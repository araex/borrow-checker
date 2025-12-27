use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    ensure_tailwind_cli();

    // Compile Tailwind CSS
    compile_tailwind();

    tauri_build::build()
}

fn ensure_tailwind_cli() {
    let bin_dir = "bin";
    fs::create_dir_all(bin_dir).expect("Failed to create bin directory");

    let (executable_name, download_url) = get_tailwind_info();
    let tailwind_path = format!("{}/{}", bin_dir, executable_name);

    if Path::new(&tailwind_path).exists() {
        println!(
            "cargo:info=Tailwind CLI already exists at {}",
            tailwind_path
        );
        return;
    }

    println!(
        "cargo:info=Downloading Tailwind CLI from {}",
        download_url
    );

    let output = Command::new("curl")
        .args(&["-sL", "-o", &tailwind_path, &download_url])
        .output()
        .expect("Failed to download Tailwind CLI");

    if !output.status.success() {
        panic!("Failed to download Tailwind CLI: {:?}", output);
    }

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tailwind_path)
            .expect("Failed to get file metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tailwind_path, perms).expect("Failed to set file permissions");
    }
}

fn get_tailwind_info() -> (String, String) {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => (
            "tailwindcss".to_string(),
            "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64".to_string()
        ),
        ("linux", "aarch64") => (
            "tailwindcss".to_string(),
            "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-arm64".to_string()
        ),
        ("macos", "aarch64") => (
            "tailwindcss".to_string(),
            "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-macos-arm64".to_string()
        ),
        ("macos", "x86_64") => (
            "tailwindcss".to_string(),
            "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-macos-x64".to_string()
        ),
        ("windows", "x86_64") => (
            "tailwindcss.exe".to_string(),
            "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-windows-x64.exe".to_string()
        ),
        _ => panic!("Unsupported platform: {} {}", os, arch)
    }
}

fn compile_tailwind() {
    let (executable_name, _) = get_tailwind_info();
    let tailwind_path = format!("bin/{}", executable_name);

    if !Path::new(&tailwind_path).exists() {
        println!("cargo:info=Tailwind CLI not found, skipping CSS compilation");
        return;
    }

    println!("cargo:info=Compiling Tailwind CSS");

    let output = Command::new(&tailwind_path)
        .args(&[
            "-i",
            "static/input.css",
            "-o",
            "static/styles.css",
            "-c",
            "tailwind.config.js",
        ])
        .output()
        .expect("Failed to run Tailwind CLI");

    if !output.status.success() {
        panic!(
            "Failed to compile Tailwind CSS: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("cargo:info=Tailwind CSS compiled successfully");

    // Tell Cargo to rerun if these files change
    println!("cargo:rerun-if-changed=static/input.css");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=static/index.html");
}
