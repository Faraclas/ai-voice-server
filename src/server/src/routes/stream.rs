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

async fn download_model(model: &str, model_dir: &str, socket: &mut WebSocket) -> Result<(), String> {
    let file_path = std::path::Path::new(model_dir).join(format!("{}.gguf", model));
    if file_path.exists() {
        return Ok(());
    }

    let _ = socket.send(Message::Text(serde_json::json!({
        "status": "downloading",
        "progress_pct": 0.0
    }).to_string().into())).await;

    tokio::fs::create_dir_all(model_dir).await.map_err(|e| format!("Failed to create model dir: {}", e))?;

    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin", model);
    let response = reqwest::get(&url).await.map_err(|e| format!("Reqwest failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    
    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| format!("Failed to create file: {}", e))?;
    let mut downloaded: u64 = 0;
    let mut last_reported_pct = -1.0;

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk).await.map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let pct = (downloaded as f64 / total_size as f64) * 100.0;
            // Report every 1% to avoid spamming the websocket
            if pct - last_reported_pct >= 1.0 {
                let _ = socket.send(Message::Text(serde_json::json!({
                    "status": "downloading",
                    "progress_pct": (pct * 10.0).round() / 10.0
                }).to_string().into())).await;
                last_reported_pct = pct;
            }
        }
    }

    let _ = socket.send(Message::Text(serde_json::json!({
        "status": "downloading",
        "progress_pct": 100.0
    }).to_string().into())).await;

    Ok(())
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut audio_buffer = Vec::new();

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Binary(data) => {
                    audio_buffer.extend(data);
                    if audio_buffer.len() > 1_920_000 {
                        println!("Audio buffer exceeded maximum size, forcing end of stream.");
                        break;
                    }
                }
                Message::Text(text) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
                            if action == "set_model" {
                                if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
                                    match download_model(model, &state.config.model_dir, &mut socket).await {
                                        Ok(_) => {
                                            match state.queue.set_model(model.to_string()).await {
                                                Ok(_) => {
                                                    let _ = socket.send(Message::Text(serde_json::json!({
                                                        "status": "success",
                                                        "message": format!("Successfully loaded {}", model)
                                                    }).to_string().into())).await;
                                                }
                                                Err(e) => {
                                                    let _ = socket.send(Message::Text(serde_json::json!({
                                                        "error": format!("Failed to load model into VRAM: {}", e)
                                                    }).to_string().into())).await;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = socket.send(Message::Text(serde_json::json!({
                                                "error": format!("Download failed: {}", e)
                                            }).to_string().into())).await;
                                        }
                                    }
                                }
                            } else if action == "end_stream" {
                                break;
                            }
                        }
                    }
                }
                Message::Close(_) => {
                    return; 
                }
                _ => {}
            }
        } else {
            return; 
        }
    }

    if !audio_buffer.is_empty() {
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
