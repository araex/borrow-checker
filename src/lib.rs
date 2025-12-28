use std::env;

use crate::git_adapter::GitPersistence;
use crate::traits::PersistenceRepository;

mod accounting;
mod commands;
mod components;
mod git_adapter;
mod ssh_keys;
mod structs;
mod traits;
mod validator;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ssh_keys::ensure_ssh_keys().expect("Failed to ensure SSH keys");

    let persistence = GitPersistence::new(None).unwrap();
    let group = persistence.load_group().unwrap();
    let ledgers = persistence.list_ledgers().unwrap();

    // @todo load from config
    let ledger_id = ledgers[0].id;
    let user_id = group.entities[0].id;
    let transactions = persistence.list_transactions(ledger_id).unwrap();

    // let mut clone = ledgers[0].clone();
    // clone.display_name = String::from("Fooo");
    // let _ = persistence.update_ledger(clone);

    tauri::Builder::default()
        .manage(structs::AppState {
            group: std::sync::Mutex::new(group),
            ledgers: std::sync::Mutex::new(ledgers),
            transactions: std::sync::Mutex::new(transactions),
            current_ledger_id: std::sync::Mutex::new(Some(ledger_id)),
            user_id,
        })
        .invoke_handler(tauri::generate_handler![
            commands::render_header,
            commands::render_ledger_header,
            commands::render_transactions,
            commands::switch_ledger,
            commands::get_expense,
            commands::render_settings,
            commands::render_main_content
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
