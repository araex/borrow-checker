use std::env;
use std::path::Path;

use crate::git_adapter::GitPersistence;
use crate::traits::PersistenceRepository;

mod commands;
mod components;
mod git_adapter;
mod structs;
mod traits;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use structs::*;
    use uuid::Uuid;

    let persistence = GitPersistence::new(None).unwrap();
    let group = persistence.load_group().unwrap();
    let ledgers = persistence.list_ledgers().unwrap();

    // @todo load from config
    let ledger_id = ledgers[0].id;
    let user_id = group.entities[0].id;
    let transactions = persistence.list_transactions(ledger_id).unwrap();

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
            commands::get_expense
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
