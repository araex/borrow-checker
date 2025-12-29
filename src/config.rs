use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

/// Application configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    /// The remote URL of the group repository
    pub group_remote_url: String,
    /// The local path where the cloned repository lives
    pub local_repo_path: Option<PathBuf>,
    /// The user's selected entity ID
    pub user_id: Option<Uuid>,
}

impl AppConfig {
    /// Create a new config with default (hardcoded) values
    pub fn default() -> Self {
        AppConfig {
            // Hardcoded for now, will be user-configurable later
            group_remote_url: String::from("https://github.com/araex/borrow-checker-testdata.git"),
            local_repo_path: None,
            user_id: None,
        }
    }
}

/// Get the path to the config directory for this application
pub fn get_config_dir() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory not found"))?;

    let app_config_dir = config_dir.join("borrow-checker");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&app_config_dir)?;

    Ok(app_config_dir)
}

/// Get the path to the config file
fn get_config_path() -> io::Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("config.toml"))
}

/// Check if the config file exists
pub fn config_exists() -> bool {
    if let Ok(config_path) = get_config_path() {
        config_path.exists()
    } else {
        false
    }
}

/// Load the configuration from disk
pub fn load_config() -> io::Result<AppConfig> {
    let config_path = get_config_path()?;
    let content = fs::read_to_string(config_path)?;

    toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

/// Save the configuration to disk
pub fn save_config(config: &AppConfig) -> io::Result<()> {
    let config_path = get_config_path()?;
    let content = toml::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    fs::write(config_path, content)?;
    Ok(())
}

/// Ensure config exists, creating it with default values if necessary
pub fn ensure_config() -> io::Result<AppConfig> {
    if config_exists() {
        log::info!("Loading existing config");
        load_config()
    } else {
        log::info!("Config not found, creating with default values");
        let config = AppConfig::default();
        save_config(&config)?;
        Ok(config)
    }
}
