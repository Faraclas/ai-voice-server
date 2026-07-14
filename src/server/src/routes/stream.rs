use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use crate::AppState;
use serde_json::Value;

pub async fn stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut audio_buffer = Vec::new();

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Binary(data) => {
                    // Collect audio chunk
                    audio_buffer.extend(data);
                }
                Message::Text(text) => {
                    // Parse control message
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
                            if action == "set_model" {
                                if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
                                    let _ = state.queue.set_model(model.to_string()).await;
                                }
                            } else if action == "end_stream" {
                                break;
                            }
                        }
                    }
                }
                Message::Close(_) => {
                    return; // Client disconnected early
                }
                _ => {}
            }
        } else {
            return; // Error receiving message
        }
    }

    // Process audio buffer
    if !audio_buffer.is_empty() {
        // Convert to f32 PCM
        let mut f32_audio = Vec::new();
        for chunk in audio_buffer.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            f32_audio.push(sample as f32 / 32768.0);
        }

        match state.queue.transcribe(f32_audio).await {
            Ok((text, processing_time_ms)) => {
                let response = serde_json::json!({
                    "text": text,
                    "is_final": true,
                    "processing_time_ms": processing_time_ms
                });
                let _ = socket.send(Message::Text(response.to_string().into())).await;
            }
            Err(e) => {
                let response = serde_json::json!({
                    "error": e
                });
                let _ = socket.send(Message::Text(response.to_string().into())).await;
            }
        }
    }
}
