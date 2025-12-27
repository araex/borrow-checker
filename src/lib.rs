use std::env;
use std::path::Path;

mod commands;
mod components;
mod git_adapter;
mod structs;

//@todo do lazy loading of transactions
fn load_ledgers() -> Vec<structs::LedgerWithTransactions> {
    let repo = git_adapter::git_adapter::get_repo();
    let mut ledgers_with_transactions: Vec<structs::LedgerWithTransactions> = Vec::new();

    match git_adapter::git_adapter::list_ledgers(&repo, Path::new("ledgers")) {
        Ok(ledgers) => {
            for (path, ledger) in ledgers {
                println!("ledger at {} : {}", path.display(), ledger.display_name);

                let item = structs::LedgerWithTransactions {
                    ledger: ledger,
                    transactions: git_adapter::git_adapter::get_transactions(&repo, &path).unwrap(),
                };

                ledgers_with_transactions.push(item);
                // path: PathBuf, ledger: structs::Ledger (owned)
            }
        }
        Err(e) => eprintln!("error: {}", e),
    }

    return ledgers_with_transactions;
}

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

    let ledgers = load_ledgers();

    // @todo load from config
    let ledger_id = ledgers[0].ledger.id;
    let user_id = test_group.entities[0].id;

    tauri::Builder::default()
        .manage(structs::AppState {
            group: std::sync::Mutex::new(test_group),
            ledgers: std::sync::Mutex::new(ledgers),
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
