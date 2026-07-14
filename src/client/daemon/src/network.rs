use anyhow::{Result, Context};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, debug};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[derive(Serialize)]
pub struct ClientMessage {
    pub action: String,
    pub model: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ServerMessage {
    pub text: Option<String>,
    pub is_final: Option<bool>,
    pub processing_time_ms: Option<u64>,
    pub status: Option<String>,
    pub progress_pct: Option<f64>,
}

pub struct NetworkClient {
    ws_url: String,
}

impl NetworkClient {
    pub fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
        }
    }

    /// Spawns a background task that manages the WebSocket connection.
    /// It consumes audio chunks from `audio_rx` and sends transcribed text to `text_tx`.
    pub async fn start(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        text_tx: mpsc::Sender<String>,
        status_tx: mpsc::Sender<(String, Option<f64>)>,
    ) -> Result<()> {
        let url = Url::parse(&self.ws_url).context("Invalid WebSocket URL")?;
        info!("Connecting to WebSocket at {}...", url);

        let (ws_stream, _) = connect_async(url.as_str()).await.context("Failed to connect to WebSocket")?;
        info!("WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // Task to read responses from the server
        let read_task = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(t)) => {
                        if let Ok(response) = serde_json::from_str::<ServerMessage>(&t) {
                            if response.is_final == Some(true) {
                                if let Some(text) = response.text {
                                    debug!("Received final transcription: {}", text);
                                    let _ = text_tx.send(text).await;
                                }
                            } else if let Some(status) = response.status {
                                let _ = status_tx.send((status, response.progress_pct)).await;
                            }
                        } else {
                            error!("Failed to parse server JSON: {}", t);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Server closed WebSocket connection.");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Loop to send audio chunks from the channel
        while let Some(chunk) = audio_rx.recv().await {
            // A zero-length chunk signals the end of the recording burst
            if chunk.is_empty() {
                debug!("End of audio burst received, sending end_stream signal.");
                let end_msg = ClientMessage {
                    action: "end_stream".to_string(),
                    model: None,
                };
                let json = serde_json::to_string(&end_msg).unwrap();
                if let Err(e) = write.send(Message::Text(json.into())).await {
                    error!("Failed to send end_stream message: {}", e);
                    break;
                }
            } else {
                if let Err(e) = write.send(Message::Binary(chunk.into())).await {
                    error!("Failed to send audio chunk over WebSocket: {}", e);
                    break;
                }
            }
        }

        info!("Network client shutting down.");
        let _ = write.close().await;
        let _ = read_task.await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use futures_util::SinkExt;

    #[test]
    fn test_server_message_deserialization() {
        // Test standard transcription payload
        let json1 = r#"{"text": "Hello world", "is_final": true, "processing_time_ms": 150}"#;
        let msg1: ServerMessage = serde_json::from_str(json1).unwrap();
        assert_eq!(msg1.text, Some("Hello world".to_string()));
        assert_eq!(msg1.is_final, Some(true));
        assert_eq!(msg1.processing_time_ms, Some(150));
        assert_eq!(msg1.status, None);

        // Test dynamic downloading payload
        let json2 = r#"{"status": "downloading", "progress_pct": 45.2}"#;
        let msg2: ServerMessage = serde_json::from_str(json2).unwrap();
        assert_eq!(msg2.text, None);
        assert_eq!(msg2.is_final, None);
        assert_eq!(msg2.status, Some("downloading".to_string()));
        assert_eq!(msg2.progress_pct, Some(45.2));
        
        // Test ready payload
        let json3 = r#"{"status": "ready"}"#;
        let msg3: ServerMessage = serde_json::from_str(json3).unwrap();
        assert_eq!(msg3.status, Some("ready".to_string()));
    }

    #[tokio::test]
    async fn test_network_client_mock_server() {
        // 1. Spin up a dummy WebSocket server on a random local port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let ws_url = format!("ws://{}/stream", addr);

        // Run the server in the background
        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                if let Ok(mut ws_stream) = accept_async(stream).await {
                    // Simulate receiving an audio chunk and immediately responding
                    if let Some(_) = ws_stream.next().await {
                        let response = r#"{"text": "Mock response", "is_final": true}"#;
                        let _ = ws_stream.send(Message::Text(response.into())).await;
                        // Intentionally close the connection
                        let _ = ws_stream.close(None).await;
                    }
                }
            }
        });

        // 2. Start the Network Client
        let client = NetworkClient::new(&ws_url);
        let (audio_tx, audio_rx) = mpsc::channel(10);
        let (text_tx, mut text_rx) = mpsc::channel(10);
        let (status_tx, _status_rx) = mpsc::channel(10);

        let client_task = tokio::spawn(async move {
            let _ = client.start(audio_rx, text_tx, status_tx).await;
        });

        // 3. Send a dummy binary chunk to trigger the server interaction, followed by an empty chunk to trigger end_stream
        let _ = audio_tx.send(vec![0x00, 0x01]).await;
        let _ = audio_tx.send(vec![]).await;
        drop(audio_tx);

        // 4. Verify we got the expected text response despite the imminent closure
        if let Some(text) = text_rx.recv().await {
            assert_eq!(text, "Mock response");
        } else {
            panic!("Did not receive expected response from mock server");
        }

        // Wait for tasks to exit cleanly without crashing
        let _ = server_task.await;
        let _ = client_task.await;
    }
}

