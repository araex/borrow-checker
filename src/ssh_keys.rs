use rand::rngs::OsRng;
use ssh_key::LineEnding;
use ssh_key::PrivateKey;
use ssh_key::private::Ed25519Keypair;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Get the path to the SSH key directory for this application
fn get_ssh_key_dir() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory not found"))?;

    let app_config_dir = config_dir.join("borrow-checker");
    let ssh_dir = app_config_dir.join("ssh");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&ssh_dir)?;

    Ok(ssh_dir)
}

/// Get the path to the private key file
fn get_private_key_path() -> io::Result<PathBuf> {
    let ssh_dir = get_ssh_key_dir()?;
    Ok(ssh_dir.join("id_ed25519"))
}

/// Get the path to the public key file
fn get_public_key_path() -> io::Result<PathBuf> {
    let ssh_dir = get_ssh_key_dir()?;
    Ok(ssh_dir.join("id_ed25519.pub"))
}

/// Check if SSH keys already exist
pub fn keys_exist() -> bool {
    if let (Ok(private_path), Ok(public_path)) = (get_private_key_path(), get_public_key_path()) {
        private_path.exists() && public_path.exists()
    } else {
        false
    }
}

/// Generate a new Ed25519 SSH key pair without a passphrase
pub fn generate_ssh_key() -> io::Result<()> {
    log::info!("Generating new SSH key pair...");

    // Generate a new Ed25519 keypair
    let keypair = Ed25519Keypair::random(&mut OsRng);
    let private_key = PrivateKey::from(keypair);

    // Get file paths
    let private_path = get_private_key_path()?;
    let public_path = get_public_key_path()?;

    // Write private key (without passphrase)
    let private_pem = private_key
        .to_openssh(LineEnding::LF)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    fs::write(&private_path, private_pem.as_bytes())?;

    // Set restrictive permissions on the private key (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&private_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&private_path, perms)?;
    }

    // Write public key
    let public_key = private_key.public_key();
    let public_ssh = public_key
        .to_openssh()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    fs::write(&public_path, public_ssh)?;

    log::info!(
        "SSH keys generated successfully at: {:?}",
        private_path.parent()
    );

    Ok(())
}

/// Ensure SSH keys exist, generating them if necessary
pub fn ensure_ssh_keys() -> io::Result<()> {
    if !keys_exist() {
        log::info!("SSH keys not found, generating new keys...");
        generate_ssh_key()?;
    } else {
        log::info!("SSH keys already exist");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        // This test actually generates keys in the config directory
        // You may want to modify this to use a temporary directory instead
        let result = ensure_ssh_keys();
        assert!(result.is_ok());
        assert!(keys_exist());
    }
}
