use std::env;
use std::fs;
use std::path::Path;
mod commands;
mod components;
mod git_adapter;
mod structs;

#[tauri::command]
fn list_files_html() -> Result<String, String> {
    git_adapter::git_adapter::get_transactions(
        git_adapter::git_adapter::get_repo(),
        Path::new("ledgers/39C3"),
    )
    .ok();
    let current_dir = env::current_dir().map_err(|e| e.to_string())?;
    let dir_path = current_dir.to_string_lossy();

    let mut entries: Vec<_> = fs::read_dir(&current_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let mut html = format!(
        r#"<div><p class="text-gray-400 text-sm mb-4 font-mono"><strong class="text-gray-200">Directory:</strong> {}</p><ul class="space-y-1">"#,
        dir_path
    );

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.metadata().ok().map(|m| m.is_dir()).unwrap_or(false);
        let icon = if is_dir { "📁" } else { "📄" };
        html.push_str(&format!(
            r#"<li class="text-gray-300"><span class="mr-2">{}</span>{}</li>"#,
            icon, name
        ));
    }

    html.push_str("</ul></div>");
    Ok(html)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use rational::Rational;
    use std::str::FromStr;
    use structs::*;
    use toml::value::Datetime;
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

    let test_transaction = Transaction {
        description: "🛫".to_string(),
        paid_by_entity: Uuid::parse_str("3abaaf40-a35a-488d-8ef2-0184c8c5f3c3").unwrap(),
        currency_iso_4217: "CHF".to_string(),
        amount: 598.80,
        transaction_datetime_rfc_3339: Datetime::from_str("2025-11-17T14:43:02Z").unwrap(),
        split_ratios: vec![
            Split {
                entity_id: Uuid::parse_str("c8744a29-7ed0-447a-af5a-51e4ad291d1d").unwrap(),
                ratio: Rational::new(1, 3),
            },
            Split {
                entity_id: Uuid::parse_str("3abaaf40-a35a-488d-8ef2-0184c8c5f3c3").unwrap(),
                ratio: Rational::new(1, 3),
            },
            Split {
                entity_id: Uuid::parse_str("92c0a0fc-aa86-4922-ab1f-7b9326720177").unwrap(),
                ratio: Rational::new(1, 3),
            },
        ],
    };

    let test_ledger_with_transactions = LedgerWithTransactions {
        ledger: test_ledger,
        transactions: vec![test_transaction],
    };

    tauri::Builder::default()
        .manage(structs::AppState {
            group: std::sync::Mutex::new(test_group),
            ledgers: std::sync::Mutex::new(vec![test_ledger_with_transactions]),
        })
        .invoke_handler(tauri::generate_handler![
            list_files_html,
            commands::render_navigation,
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
