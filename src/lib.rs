use std::env;
use std::fs;
mod git_adapter;
mod structs;

#[tauri::command]
fn list_files_html() -> Result<String, String> {
    git_adapter::git_backend::get_transactions().ok();
    let current_dir = env::current_dir().map_err(|e| e.to_string())?;
    let dir_path = current_dir.to_string_lossy();

    let mut entries: Vec<_> = fs::read_dir(&current_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let mut html = format!(r#"<p><strong>Directory:</strong> {}</p><ul>"#, dir_path);

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.metadata().ok().map(|m| m.is_dir()).unwrap_or(false);
        let icon = if is_dir { "📁" } else { "📄" };
        html.push_str(&format!(
            r#"<li><span class="file-icon">{}</span> {}</li>"#,
            icon, name
        ));
    }

    html.push_str("</ul>");
    Ok(html)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_files_html])
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
