use crate::config::get_config_dir;
use crate::ssh_keys::get_private_key_path;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Manages repository cloning, validation, and directory management
pub struct RepoManager;

impl RepoManager {
    /// Clone a repository to a hashed directory under ~/.config/borrow-checker/repositories/
    /// Returns the path to the cloned repository
    pub fn clone_repository(url: &str) -> Result<PathBuf, String> {
        // Validate URL format
        Self::validate_url(url)?;

        // Calculate hash for directory name
        let repo_dir = Self::get_repo_directory(url)?;

        // Remove existing directory if it exists
        if repo_dir.exists() {
            fs::remove_dir_all(&repo_dir)
                .map_err(|e| format!("Failed to remove existing repository: {}", e))?;
        }

        // Check if SSH keys exist
        let private_key_path =
            get_private_key_path().map_err(|e| format!("Failed to get private key path: {}", e))?;

        if !private_key_path.exists() {
            return Err(format!(
                "SSH private key not found at {}",
                private_key_path.display()
            ));
        }

        // Set up SSH authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            log::info!("Authenticating with SSH key");
            Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                &private_key_path,
                None,
            )
        });

        // Accept any host key (skip host key verification)
        callbacks.certificate_check(|_cert, _host| {
            log::info!("Accepting host certificate");
            Ok(git2::CertificateCheckStatus::CertificateOk)
        });

        // Add transfer progress callback
        callbacks.transfer_progress(|stats| {
            log::info!(
                "Received {}/{} objects",
                stats.received_objects(),
                stats.total_objects()
            );
            true
        });

        // Configure fetch options
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Build clone options
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        // Clone the repository
        log::info!("Cloning repository from {} to {}", url, repo_dir.display());
        builder.clone(url, &repo_dir)
            .map_err(|e| format!("Failed to clone repository: {}. Make sure your SSH key is added to your Git provider.", e))?;

        log::info!("Successfully cloned repository");
        Ok(repo_dir)
    }

    /// Validate that the cloned repository has the required structure
    pub fn validate_repo_structure(repo_path: &PathBuf) -> Result<(), String> {
        // Check for group.toml
        let group_toml = repo_path.join("group.toml");
        if !group_toml.exists() {
            return Err("Repository is missing group.toml file".to_string());
        }

        // Validate group.toml can be parsed
        let content = fs::read_to_string(&group_toml)
            .map_err(|e| format!("Failed to read group.toml: {}", e))?;

        toml::from_str::<toml::Value>(&content)
            .map_err(|e| format!("Invalid group.toml format: {}", e))?;

        // Check for ledgers directory
        let ledgers_dir = repo_path.join("ledgers");
        if !ledgers_dir.exists() {
            return Err("Repository is missing ledgers/ directory".to_string());
        }

        if !ledgers_dir.is_dir() {
            return Err("ledgers/ exists but is not a directory".to_string());
        }

        log::info!("Repository structure validated successfully");
        Ok(())
    }

    /// Get the directory path for a repository based on URL hash
    fn get_repo_directory(url: &str) -> Result<PathBuf, String> {
        let config_dir =
            get_config_dir().map_err(|e| format!("Failed to get config directory: {}", e))?;

        let repos_dir = config_dir.join("repositories");
        fs::create_dir_all(&repos_dir)
            .map_err(|e| format!("Failed to create repositories directory: {}", e))?;

        // Hash the URL to create a unique directory name
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        let dir_name = format!("{:x}", hash);

        Ok(repos_dir.join(dir_name))
    }

    /// Validate URL format (prevent file:// URLs and other potential security issues)
    fn validate_url(url: &str) -> Result<(), String> {
        let url_lower = url.to_lowercase();

        if url_lower.starts_with("file://") {
            return Err("file:// URLs are not allowed".to_string());
        }

        if !url_lower.starts_with("http://")
            && !url_lower.starts_with("https://")
            && !url_lower.starts_with("git@")
            && !url_lower.starts_with("ssh://")
        {
            return Err(
                "Invalid URL format. Must be http://, https://, git@, or ssh://".to_string(),
            );
        }

        if url.trim().is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_https() {
        assert!(RepoManager::validate_url("https://github.com/user/repo.git").is_ok());
    }

    #[test]
    fn test_validate_url_ssh() {
        assert!(RepoManager::validate_url("git@github.com:user/repo.git").is_ok());
    }

    #[test]
    fn test_validate_url_file_rejected() {
        assert!(RepoManager::validate_url("file:///path/to/repo").is_err());
    }

    #[test]
    fn test_validate_url_empty_rejected() {
        assert!(RepoManager::validate_url("").is_err());
    }
}
