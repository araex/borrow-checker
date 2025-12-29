use std::env;

use crate::git_adapter::GitPersistence;
use crate::traits::PersistenceRepository;

mod accounting;
mod api_commands;
mod api_types;
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
    let (app_state, persistence_instance) = if config.local_repo_path.is_some() {
        eprintln!("[INFO] User is onboarded, loading group data");
        eprintln!("[INFO] Using group remote URL: {}", config.group_remote_url);

        let repo_path = config.local_repo_path.clone()
            .expect("local_repo_path must be set when is_onboarded is true");

        let persistence = GitPersistence::new(repo_path.clone()).unwrap();
        let group = persistence.load_group().unwrap();
        let ledgers = persistence.list_ledgers().unwrap();

        // Use first ledger as default
        let ledger_id = ledgers[0].id;
        
        // Use user_id from config if available
        let user_id = config.user_id;
        
        let transactions = persistence.list_transactions(ledger_id).unwrap();

        let app = structs::AppState {
            config: std::sync::Mutex::new(config),
            group: std::sync::Mutex::new(Some(group)),
            ledgers: std::sync::Mutex::new(Some(ledgers)),
            transactions: std::sync::Mutex::new(Some(transactions)),
            current_ledger_id: std::sync::Mutex::new(Some(ledger_id)),
            user_id: std::sync::Mutex::new(user_id),
        };
        
        (app, Some(GitPersistence::new(repo_path).unwrap()))
    } else {
        eprintln!("[INFO] User not onboarded, starting with empty state");
        
        let app = structs::AppState {
            config: std::sync::Mutex::new(config),
            group: std::sync::Mutex::new(None),
            ledgers: std::sync::Mutex::new(None),
            transactions: std::sync::Mutex::new(None),
            current_ledger_id: std::sync::Mutex::new(None),
            user_id: std::sync::Mutex::new(None),
        };
        
        (app, None)
    };

    let mut builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .manage(app_state);
    
    // Only manage persistence if user is onboarded
    if let Some(persistence) = persistence_instance {
        builder = builder.manage(persistence);
    }
    
    builder
        .invoke_handler(tauri::generate_handler![
            api_commands::get_ssh_public_key,
            api_commands::is_onboarded,
            api_commands::join_group,
            api_commands::get_entities,
            api_commands::select_entity,
            api_commands::add_new_entity,
            api_commands::get_app_state,
            api_commands::get_transactions,
            api_commands::switch_ledger,
            api_commands::get_expense,
            api_commands::create_expense,
            api_commands::update_expense,
            api_commands::delete_expense,
            api_commands::get_settings,
            api_commands::reset_user,
        ])
        .setup(|_app| {
            log::info!("Tauri application setup complete - logging is now active");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
