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
    pub use_gpu: bool,
    pub active_device: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("PORT must be a valid u16");

        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
        
        let whisper_model = std::env::var("WHISPER_MODEL").unwrap_or_else(|_| "small.en".to_string());
        
        let model_dir = std::env::var("MODEL_DIR").unwrap_or_else(|_| "./models".to_string());
        
        let max_queue_depth = std::env::var("MAX_QUEUE_DEPTH")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .expect("MAX_QUEUE_DEPTH must be a valid usize");
            
        let gpu_mode_str = std::env::var("GPU_MODE").unwrap_or_else(|_| "auto".to_string());
        let gpu_mode = match gpu_mode_str.to_lowercase().as_str() {
            "require" => GpuMode::Require,
            _ => GpuMode::Auto,
        };
        
        let device_priority_str = std::env::var("DEVICE_PRIORITY").unwrap_or_else(|_| "cuda,vulkan,cpu".to_string());
        let device_priority: Vec<String> = device_priority_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        // Hardware Probing (runs exactly once at startup)
        let mut use_gpu = false;
        let mut active_device = "cpu".to_string();

        for dev in &device_priority {
            match dev.as_str() {
                "cuda" | "nvidia" => {
                    if std::process::Command::new("nvidia-smi").output().is_ok() {
                        use_gpu = true;
                        active_device = "cuda".to_string();
                        break;
                    }
                }
                "rocm" | "hip" => {
                    if std::process::Command::new("rocm-smi").output().is_ok() {
                        use_gpu = true;
                        active_device = "rocm".to_string();
                        break;
                    }
                }
                "vulkan" => {
                    if std::process::Command::new("vulkaninfo").output().is_ok() {
                        use_gpu = true;
                        active_device = "vulkan".to_string();
                        break;
                    }
                }
                "cpu" => {
                    use_gpu = false;
                    active_device = "cpu".to_string();
                    break;
                }
                _ => {}
            }
        }

        if !use_gpu && gpu_mode == GpuMode::Require {
            panic!("GPU is required by config but no valid GPU was detected.");
        }

        Self {
            port,
            bind_addr,
            whisper_model,
            model_dir,
            max_queue_depth,
            gpu_mode,
            device_priority,
            use_gpu,
            active_device,
        }
    }
}
