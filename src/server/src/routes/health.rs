use axum::{Json, extract::State};
use serde::Serialize;
use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub gpu_active: bool,
    pub loaded_model: String,
}

pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    match state.queue.get_status().await {
        Ok(resp) => Json(resp),
        Err(e) => Json(HealthResponse {
            status: format!("error: {}", e),
            gpu_active: false,
            loaded_model: "".to_string(),
        }),
    }
}
