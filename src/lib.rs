use std::env;
use std::fs;
mod commands;
mod components;
mod structs;

#[tauri::command]
fn list_files_html() -> Result<String, String> {
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
    tauri::Builder::default()
        .manage(structs::AppState {
            current_group: std::sync::Mutex::new("CCC Events".to_string()),
            current_ledger: std::sync::Mutex::new("39C3".to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            list_files_html,
            commands::render_navigation,
            commands::switch_group,
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
