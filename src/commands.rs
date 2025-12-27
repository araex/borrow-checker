use crate::components::Navigation;
use crate::structs::AppState;
use uuid::Uuid;

#[tauri::command]
pub fn render_navigation(state: tauri::State<AppState>) -> Result<String, String> {
    // For now, we're displaying the first group's name (since we only support 1 group)
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    
    // Use "No Group" as placeholder if entities are empty
    let group_name = if group.entities.is_empty() {
        "No Group".to_string()
    } else {
        "Current Group".to_string()
    };
    
    // Use first ledger's name if available, otherwise placeholder
    let ledger_name = ledgers
        .first()
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "No Ledger".to_string());

    let nav = Navigation::new()
        .current_group(&group_name)
        .current_ledger(&ledger_name)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn switch_group(_group_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    // For now, since we only support 1 group, this is mostly a placeholder
    // In the future, this could load a different group from disk
    render_navigation(state)
}

#[tauri::command]
pub fn switch_ledger(ledger_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    // Parse the ledger_id as UUID and find the matching ledger
    let uuid = Uuid::parse_str(&ledger_id).map_err(|e| e.to_string())?;
    
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    
    // Find the ledger with the matching ID
    let ledger_name = ledgers
        .iter()
        .find(|l| l.ledger.id == uuid)
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "Unknown Ledger".to_string());
    
    let group_name = if group.entities.is_empty() {
        "No Group".to_string()
    } else {
        "Current Group".to_string()
    };

    let nav = Navigation::new()
        .current_group(&group_name)
        .current_ledger(&ledger_name)
        .build();

    Ok(nav)
}
