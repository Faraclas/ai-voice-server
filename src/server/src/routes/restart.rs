use axum::{Json, extract::State, http::HeaderMap};
use serde::Serialize;
use crate::AppState;

#[derive(Serialize)]
pub struct RestartResponse {
    pub status: String,
    pub message: String,
}

pub async fn restart_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<RestartResponse> {
    let expected_key = match &state.config.admin_api_key {
        Some(k) => k,
        None => return Json(RestartResponse {
            status: "error".to_string(),
            message: "ADMIN_API_KEY not configured on server".to_string(),
        }),
    };

    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    let is_valid = match auth_header {
        Some(header) => header == format!("Bearer {}", expected_key),
        None => false,
    };

    if !is_valid {
        return Json(RestartResponse {
            status: "error".to_string(),
            message: "Unauthorized".to_string(),
        });
    }

    // Spawn a background task to exit the process after returning the HTTP response
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        println!("Received remote restart request from authenticated admin. Exiting process (exit status 42) for systemd restart...");
        std::process::exit(42);
    });

    Json(RestartResponse {
        status: "success".to_string(),
        message: "Server service restart initiated.".to_string(),
    })
}
