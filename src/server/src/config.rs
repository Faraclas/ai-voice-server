use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuMode {
    Auto,
    Require,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub bind_addr: String,
    pub whisper_model: String,
    pub model_dir: String,
    pub max_queue_depth: usize,
    pub gpu_mode: GpuMode,
    pub device_priority: Vec<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("PORT must be a valid u16");

        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
        
        let whisper_model = env::var("WHISPER_MODEL").unwrap_or_else(|_| "small.en".to_string());
        
        let model_dir = env::var("MODEL_DIR").unwrap_or_else(|_| "./models".to_string());
        
        let max_queue_depth = env::var("MAX_QUEUE_DEPTH")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .expect("MAX_QUEUE_DEPTH must be a valid usize");
            
        let gpu_mode_str = env::var("GPU_MODE").unwrap_or_else(|_| "auto".to_string());
        let gpu_mode = match gpu_mode_str.to_lowercase().as_str() {
            "require" => GpuMode::Require,
            _ => GpuMode::Auto,
        };
        
        let device_priority_str = env::var("DEVICE_PRIORITY").unwrap_or_else(|_| "cuda,vulkan,cpu".to_string());
        let device_priority = device_priority_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            port,
            bind_addr,
            whisper_model,
            model_dir,
            max_queue_depth,
            gpu_mode,
            device_priority,
        }
    }
}
