use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub _config_dir: PathBuf,
    pub _data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub db_path: PathBuf,
    pub ollama_url: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let proj_dirs =
            ProjectDirs::from("", "", "vicuna").context("Could not determine config directory")?;

        let _config_dir = proj_dirs.config_dir().to_path_buf();
        let _data_dir = proj_dirs.data_dir().to_path_buf();

        std::fs::create_dir_all(&_config_dir).context("Failed to create config dir")?;
        std::fs::create_dir_all(&_data_dir).context("Failed to create data dir")?;

        let log_dir = _config_dir.join("logs");
        std::fs::create_dir_all(&log_dir).context("Failed to create log dir")?;

        let db_path = _config_dir.join("vicuna.db");
        let ollama_url =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

        Ok(Self {
            _config_dir,
            _data_dir,
            log_dir,
            db_path,
            ollama_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert!(cfg.db_path.to_string_lossy().contains("vicuna.db"));
    }
}
