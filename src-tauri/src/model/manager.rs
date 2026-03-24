use serde::Serialize;
use std::path::PathBuf;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ModelSize {
    Small,
    Medium,
    Large,
}

impl ModelSize {
    pub fn model_filename(&self) -> &'static str {
        match self {
            ModelSize::Small => "ggml-small.bin",
            ModelSize::Medium => "ggml-medium.bin",
            ModelSize::Large => "ggml-large-v3.bin",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelSize::Small => "whisper-small (~500MB)",
            ModelSize::Medium => "whisper-medium (~1.5GB)",
            ModelSize::Large => "whisper-large-v3 (~3GB)",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub recommended: ModelSize,
    pub system_ram_gb: u64,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub size: ModelSize,
    pub name: String,
    pub downloaded: bool,
    pub path: Option<String>,
}

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Self {
        let models_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("meeting-app")
            .join("models");
        std::fs::create_dir_all(&models_dir).ok();
        Self { models_dir }
    }

    /// Detect system RAM and recommend optimal model size.
    pub fn recommended_model() -> ModelSize {
        let sys = System::new_all();
        let total_ram_gb = sys.total_memory() / (1024 * 1024 * 1024);

        if total_ram_gb >= 32 {
            ModelSize::Large
        } else if total_ram_gb >= 16 {
            ModelSize::Medium
        } else {
            ModelSize::Small
        }
    }

    pub fn system_ram_gb() -> u64 {
        let sys = System::new_all();
        sys.total_memory() / (1024 * 1024 * 1024)
    }

    /// Get current model status.
    pub fn get_status(&self) -> ModelStatus {
        let recommended = Self::recommended_model();
        let sizes = [ModelSize::Small, ModelSize::Medium, ModelSize::Large];
        let models = sizes
            .iter()
            .map(|size| {
                let path = self.model_path(*size);
                let downloaded = path.exists();
                ModelInfo {
                    size: *size,
                    name: size.display_name().to_string(),
                    downloaded,
                    path: if downloaded {
                        Some(path.to_string_lossy().to_string())
                    } else {
                        None
                    },
                }
            })
            .collect();

        ModelStatus {
            recommended,
            system_ram_gb: Self::system_ram_gb(),
            models,
        }
    }

    /// Get the path where a model should be stored.
    pub fn model_path(&self, size: ModelSize) -> PathBuf {
        self.models_dir.join(size.model_filename())
    }

    /// Check if a model is downloaded.
    pub fn is_downloaded(&self, size: ModelSize) -> bool {
        self.model_path(size).exists()
    }

    pub fn models_dir(&self) -> &PathBuf {
        &self.models_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(ModelSize::Small.model_filename(), "ggml-small.bin");
        assert_eq!(ModelSize::Medium.model_filename(), "ggml-medium.bin");
        assert_eq!(ModelSize::Large.model_filename(), "ggml-large-v3.bin");
    }

    #[test]
    fn test_system_ram_detection() {
        let ram = ModelManager::system_ram_gb();
        assert!(ram > 0, "System RAM should be > 0 GB");
    }

    #[test]
    fn test_recommended_model() {
        let model = ModelManager::recommended_model();
        // On a development machine, this should return something valid
        assert!(
            model == ModelSize::Small
                || model == ModelSize::Medium
                || model == ModelSize::Large
        );
    }
}
