use std::env;
use std::path::Path;
mod commands;
mod components;
mod git_adapter;
mod structs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use structs::*;
    use uuid::Uuid;

    // Initialize test group with entities
    let test_group = Group {
        entities: vec![
            Entity {
                id: Uuid::parse_str("c8744a29-7ed0-447a-af5a-51e4ad291d1d").unwrap(),
                display_name: "Araex".to_string(),
            },
            Entity {
                id: Uuid::parse_str("3abaaf40-a35a-488d-8ef2-0184c8c5f3c3").unwrap(),
                display_name: "Wüstenschiff".to_string(),
            },
            Entity {
                id: Uuid::parse_str("92c0a0fc-aa86-4922-ab1f-7b9326720177").unwrap(),
                display_name: "flakmonkey".to_string(),
            },
        ],
    };

    // Initialize test ledger with transactions
    let test_ledger = Ledger {
        id: Uuid::parse_str("10cc6659-531e-4c8f-881f-1bf6b24abbc0").unwrap(),
        display_name: "39C3".to_string(),
        participants: vec![
            Uuid::parse_str("c8744a29-7ed0-447a-af5a-51e4ad291d1d").unwrap(),
            Uuid::parse_str("3abaaf40-a35a-488d-8ef2-0184c8c5f3c3").unwrap(),
            Uuid::parse_str("92c0a0fc-aa86-4922-ab1f-7b9326720177").unwrap(),
        ],
    };

    let test_ledger_with_transactions = LedgerWithTransactions {
        ledger: test_ledger.clone(),
        transactions: git_adapter::git_adapter::get_transactions(
            git_adapter::git_adapter::get_repo(),
            Path::new("ledgers/39C3"),
        )
        .unwrap(),
    };

    let ledger_id = test_ledger.id;
    let user_id = test_group.entities[0].id;

    tauri::Builder::default()
        .manage(structs::AppState {
            group: std::sync::Mutex::new(test_group),
            ledgers: std::sync::Mutex::new(vec![test_ledger_with_transactions]),
            current_ledger_id: std::sync::Mutex::new(Some(ledger_id)),
            user_id,
        })
        .invoke_handler(tauri::generate_handler![
            commands::render_header,
            commands::render_ledger_header,
            commands::render_transactions,
            commands::switch_ledger
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
