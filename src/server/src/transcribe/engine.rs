use crate::config::AppConfig;
use whisper_rs::{FullParams, WhisperContext, WhisperContextParameters};
use std::path::Path;

pub struct WhisperEngine {
    context: WhisperContext,
    pub use_gpu: bool,
}

impl WhisperEngine {
    pub fn new(config: &AppConfig, model_name: &str) -> Result<Self, String> {
        let use_gpu = config.use_gpu;
        let active_device = &config.active_device;

        // Try .bin or .gguf since we use GGUF now
        let mut model_path = Path::new(&config.model_dir).join(format!("{}.gguf", model_name));
        if !model_path.exists() {
            model_path = Path::new(&config.model_dir).join(format!("{}.bin", model_name));
        }
        
        if !model_path.exists() {
            return Err(format!("Model file not found at: {:?}", model_path));
        }

        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu = use_gpu;

        println!("Loading model {} (use_gpu: {}, active_device: {})", model_name, use_gpu, active_device);

        let context = WhisperContext::new_with_params(model_path.to_str().unwrap(), ctx_params)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        Ok(Self { context, use_gpu })
    }

    pub fn transcribe(&self, audio_data: &[f32]) -> Result<(String, u64), String> {
        let mut state = self.context.create_state().map_err(|e| format!("Failed to create state: {}", e))?;
        let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let start_time = std::time::Instant::now();

        state.full(params, audio_data).map_err(|e| format!("Inference failed: {}", e))?;

        let num_segments = state.full_n_segments();
        let mut result = String::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment.to_str_lossy().map_err(|e| format!("Failed to get segment text: {:?}", e))?;
                result.push_str(&text);
            } else {
                return Err("Failed to get segment".to_string());
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok((result.trim().to_string(), processing_time_ms))
    }
}
