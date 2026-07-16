
use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use crate::AppState;

#[derive(Deserialize)]
pub struct SetModelRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct SetModelResponse {
    pub status: String,
    pub loaded_model: String,
}

async fn download_model_http(model: &str, model_dir: &str) -> Result<(), String> {
    let file_path = std::path::Path::new(model_dir).join(format!("{}.bin", model));
    if file_path.exists() {
        return Ok(());
    }

    tokio::fs::create_dir_all(model_dir).await.map_err(|e| format!("Failed to create model dir: {}", e))?;

    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin", model);
    let response = reqwest::get(&url).await.map_err(|e| format!("Reqwest failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| format!("Failed to create file: {}", e))?;
    
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk).await.map_err(|e| format!("Write error: {}", e))?;
    }

    Ok(())
}

pub async fn set_model_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetModelRequest>,
) -> Json<SetModelResponse> {
    let expected_key = match &state.config.admin_api_key {
        Some(k) => k,
        None => return Json(SetModelResponse {
            status: "error: ADMIN_API_KEY not configured on server".to_string(),
            loaded_model: "".to_string(),
        }),
    };

    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    let is_valid = match auth_header {
        Some(header) => header == format!("Bearer {}", expected_key),
        None => false,
    };

    if !is_valid {
        return Json(SetModelResponse {
            status: "error: Unauthorized".to_string(),
            loaded_model: "".to_string(),
        });
    }

    // Attempt to download the model first if it doesn't exist
    if let Err(e) = download_model_http(&payload.model, &state.config.model_dir).await {
        return Json(SetModelResponse {
            status: format!("error downloading model: {}", e),
            loaded_model: "".to_string(),
        });
    }

    match state.queue.set_model(payload.model.clone()).await {
        Ok(_) => Json(SetModelResponse {
            status: "success".to_string(),
            loaded_model: payload.model,
        }),
        Err(e) => Json(SetModelResponse {
            status: format!("error loading model into VRAM: {}", e),
            loaded_model: "".to_string(),
        }),
    }
}
