use std::env;

use crate::git_adapter::GitPersistence;
use crate::traits::PersistenceRepository;

mod accounting;
mod commands;
mod components;
mod config;
mod git_adapter;
mod repo_manager;
mod ssh_keys;
mod structs;
mod traits;
mod validator;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ensure SSH keys exist
    ssh_keys::ensure_ssh_keys().expect("Failed to ensure SSH keys");

    // Ensure config exists (with hardcoded defaults for now)
    let config = config::ensure_config().expect("Failed to ensure config");
    
    // Conditional initialization based on onboarding status
    let app_state = if config.local_repo_path.is_some() {
        eprintln!("[INFO] User is onboarded, loading group data");
        eprintln!("[INFO] Using group remote URL: {}", config.group_remote_url);

        let repo_path = config.local_repo_path.clone()
            .expect("local_repo_path must be set when is_onboarded is true");

        let persistence = GitPersistence::new(Some(repo_path)).unwrap();
        let group = persistence.load_group().unwrap();
        let ledgers = persistence.list_ledgers().unwrap();

        // @todo load from config
        let ledger_id = ledgers[0].id;
        let user_id = group.entities[0].id;
        let transactions = persistence.list_transactions(ledger_id).unwrap();

        structs::AppState {
            config: std::sync::Mutex::new(config),
            group: std::sync::Mutex::new(Some(group)),
            ledgers: std::sync::Mutex::new(Some(ledgers)),
            transactions: std::sync::Mutex::new(Some(transactions)),
            current_ledger_id: std::sync::Mutex::new(Some(ledger_id)),
            user_id: Some(user_id),
        }
    } else {
        eprintln!("[INFO] User not onboarded, starting with empty state");
        
        structs::AppState {
            config: std::sync::Mutex::new(config),
            group: std::sync::Mutex::new(None),
            ledgers: std::sync::Mutex::new(None),
            transactions: std::sync::Mutex::new(None),
            current_ledger_id: std::sync::Mutex::new(None),
            user_id: None,
        }
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::render_header,
            commands::render_ledger_header,
            commands::render_transactions,
            commands::switch_ledger,
            commands::get_expense,
            commands::render_settings,
            commands::render_main_content,
            commands::get_ssh_public_key,
            commands::is_onboarded,
            commands::join_group,
        ])
        .setup(|_app| {
            log::info!("Tauri application setup complete - logging is now active");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
