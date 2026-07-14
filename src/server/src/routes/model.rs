use axum::{Json, extract::State};
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

pub async fn set_model_handler(
    State(state): State<AppState>,
    Json(payload): Json<SetModelRequest>,
) -> Json<SetModelResponse> {
    match state.queue.set_model(payload.model.clone()).await {
        Ok(_) => Json(SetModelResponse {
            status: "success".to_string(),
            loaded_model: payload.model,
        }),
        Err(e) => Json(SetModelResponse {
            status: format!("error: {}", e),
            loaded_model: "".to_string(),
        }),
    }
}
