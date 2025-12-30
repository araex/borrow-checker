use std::path::Path;
use std::process::Command;

fn main() {
    // Check if node_modules exists, if not run bun install
    let node_modules = Path::new("node_modules");
    if !node_modules.exists() {
        println!("cargo:warning=node_modules not found, running bun install...");

        let output = Command::new("bun")
            .arg("install")
            .output()
            .expect("Failed to execute bun install. Make sure bun is installed.");

        if !output.status.success() {
            panic!(
                "bun install failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("cargo:warning=bun install completed successfully");
    }

    // Tell cargo to re-run this build script if package.json changes
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=bun.lockb");

    tauri_build::build()
}
