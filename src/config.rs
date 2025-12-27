use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub db_path: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self> {
        let proj_dirs =
            ProjectDirs::from("", "", "vicuna").context("Could not determine config directory")?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        let data_dir = proj_dirs.data_dir().to_path_buf();

        std::fs::create_dir_all(&config_dir).context("Failed to create config dir")?;
        std::fs::create_dir_all(&data_dir).context("Failed to create data dir")?;

        let log_dir = config_dir.join("logs");
        std::fs::create_dir_all(&log_dir).context("Failed to create log dir")?;

        let db_path = config_dir.join("vicuna.db");

        Ok(Self {
            config_dir,
            data_dir,
            log_dir,
            db_path,
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
