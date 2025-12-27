use crate::components::{Navigation, Transaction};
use crate::structs::AppState;
use uuid::Uuid;

#[tauri::command]
pub fn render_navigation(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    
    // Use first ledger's name if available, otherwise placeholder
    let ledger_name = ledgers
        .first()
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "No Ledger".to_string());

    let nav = Navigation::new()
        .current_ledger(&ledger_name)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn switch_ledger(ledger_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    // Parse the ledger_id as UUID and find the matching ledger
    let uuid = Uuid::parse_str(&ledger_id).map_err(|e| e.to_string())?;
    
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    
    // Find the ledger with the matching ID
    let ledger_name = ledgers
        .iter()
        .find(|l| l.ledger.id == uuid)
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "Unknown Ledger".to_string());

    let nav = Navigation::new()
        .current_ledger(&ledger_name)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn render_transactions(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    
    // Get current ledger and user from state
    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;
    let user_uuid = state.user_id;
    
    // Find the ledger
    let ledger_with_txns = ledgers
        .iter()
        .find(|l| l.ledger.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;
    
    let mut html = String::from(r#"<section id="expense-list" class="flex flex-col">"#);
    
    // Render each transaction
    for txn in &ledger_with_txns.transactions {
        // Find the payer's name
        let payer_name = group
            .entities
            .iter()
            .find(|e| e.id == txn.paid_by_entity)
            .map(|e| e.display_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        
        // Calculate user's share
        let user_ratio = txn
            .split_ratios
            .iter()
            .find(|s| s.entity_id == user_uuid)
            .map(|s| s.ratio.numerator() as f64 / s.ratio.denominator() as f64)
            .unwrap_or(0.0);
        
        let user_share = txn.amount * user_ratio;
        
        // Format date
        let date = format!("{}", txn.transaction_datetime_rfc_3339);
        let date_short = date.split('T').next().unwrap_or(&date);
        
        // Build transaction component
        let mut transaction = Transaction::new()
            .description(&txn.description)
            .payer_name(&payer_name)
            .total_amount(txn.amount)
            .currency(&txn.currency_iso_4217)
            .date(date_short);
        
        // Determine if user borrowed or lent
        if txn.paid_by_entity == user_uuid {
            // User paid, so they lent money (user_share - amount)
            let lent_amount = txn.amount - user_share;
            if lent_amount > 0.01 {
                transaction = transaction.lent(lent_amount);
            }
        } else {
            // Someone else paid, user borrowed their share
            if user_share > 0.01 {
                transaction = transaction.borrowed(user_share);
            }
        }
        
        html.push_str(&transaction.build());
    }
    
    html.push_str("</section>");
    
    Ok(html)
}
