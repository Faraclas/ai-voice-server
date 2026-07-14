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
