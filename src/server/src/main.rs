mod config;
mod routes;
mod transcribe;

use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use config::AppConfig;
use transcribe::queue::JobQueue;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub queue: JobQueue,
}

#[tokio::main]
async fn main() {
    // Attempt to load a .env file for local testing. 
    // If it's not found (like in production), this silently fails and moves on!
    dotenvy::dotenv().ok();

    let config = Arc::new(AppConfig::load());
    let queue = JobQueue::new(config.clone());
    
    let state = AppState { config: config.clone(), queue };

    let app = Router::new()
        .route("/health", get(routes::health::health_handler))
        .route("/stream", get(routes::stream::stream_handler))
        .route("/set_model", post(routes::model::set_model_handler))
        .with_state(state);

    let addr = format!("{}:{}", config.bind_addr, config.port);
    println!("Starting AI Voice Server (v2) on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
