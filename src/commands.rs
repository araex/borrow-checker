use crate::components::Navigation;
use crate::structs::AppState;

#[tauri::command]
pub fn render_navigation(state: tauri::State<AppState>) -> Result<String, String> {
    let group = state.current_group.lock().map_err(|e| e.to_string())?;
    let ledger = state.current_ledger.lock().map_err(|e| e.to_string())?;

    let nav = Navigation::new()
        .current_group(&*group)
        .current_ledger(&*ledger)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn switch_group(group_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    let mut group = state.current_group.lock().map_err(|e| e.to_string())?;
    *group = group_id.clone();
    drop(group);

    let ledger = state.current_ledger.lock().map_err(|e| e.to_string())?;
    let nav = Navigation::new()
        .current_group(&group_id)
        .current_ledger(&*ledger)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn switch_ledger(ledger_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    let mut ledger = state.current_ledger.lock().map_err(|e| e.to_string())?;
    *ledger = ledger_id.clone();
    drop(ledger);

    let group = state.current_group.lock().map_err(|e| e.to_string())?;
    let nav = Navigation::new()
        .current_group(&*group)
        .current_ledger(&ledger_id)
        .build();

    Ok(nav)
}
