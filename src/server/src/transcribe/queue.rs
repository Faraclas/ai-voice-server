use crate::config::AppConfig;
use crate::transcribe::engine::WhisperEngine;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub enum WorkerCommand {
    Transcribe {
        audio: Vec<f32>,
        responder: oneshot::Sender<Result<(String, u64), String>>,
    },
    SetModel {
        model: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    GetStatus {
        responder: oneshot::Sender<crate::routes::health::HealthResponse>,
    },
}

#[derive(Clone)]
pub struct JobQueue {
    sender: mpsc::Sender<WorkerCommand>,
}

impl JobQueue {
    pub fn new(config: Arc<AppConfig>) -> Self {
        let (tx, mut rx) = mpsc::channel(config.max_queue_depth);
        let config_clone = config.clone();

        std::thread::spawn(move || {
            let mut current_model = config_clone.whisper_model.clone();
            let mut engine = match WhisperEngine::new(&config_clone, &current_model) {
                Ok(e) => Some(e),
                Err(err) => {
                    eprintln!("Failed to load initial model: {}", err);
                    None
                }
            };
            
            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    WorkerCommand::Transcribe { audio, responder } => {
                        let res = if let Some(ref e) = engine {
                            e.transcribe(&audio)
                        } else {
                            Err("Engine is not loaded".to_string())
                        };
                        let _ = responder.send(res);
                    }
                    WorkerCommand::SetModel { model, responder } => {
                        match WhisperEngine::new(&config_clone, &model) {
                            Ok(new_engine) => {
                                engine = Some(new_engine);
                                current_model = model;
                                let _ = responder.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = responder.send(Err(e));
                            }
                        }
                    }
                    WorkerCommand::GetStatus { responder } => {
                        let gpu_active = engine.as_ref().map(|e| e.use_gpu).unwrap_or(false);
                        let _ = responder.send(crate::routes::health::HealthResponse {
                            status: if engine.is_some() { "ready".to_string() } else { "error".to_string() },
                            gpu_active,
                            loaded_model: current_model.clone(),
                        });
                    }
                }
            }
        });

        Self { sender: tx }
    }

    pub async fn transcribe(&self, audio: Vec<f32>) -> Result<(String, u64), String> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(WorkerCommand::Transcribe { audio, responder: tx })
            .await
            .map_err(|_| "Queue full or worker died".to_string())?;
        rx.await.map_err(|_| "Worker disconnected".to_string())?
    }

    pub async fn set_model(&self, model: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(WorkerCommand::SetModel { model, responder: tx })
            .await
            .map_err(|_| "Queue full or worker died".to_string())?;
        rx.await.map_err(|_| "Worker disconnected".to_string())?
    }

    pub async fn get_status(&self) -> Result<crate::routes::health::HealthResponse, String> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(WorkerCommand::GetStatus { responder: tx })
            .await
            .map_err(|_| "Queue full or worker died".to_string())?;
        rx.await.map_err(|_| "Worker disconnected".to_string())
    }
}
