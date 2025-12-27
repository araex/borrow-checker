use crate::components::{Header, LedgerHeader, Transaction};
use crate::structs::AppState;
use uuid::Uuid;

#[tauri::command]
pub fn render_header(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let user_uuid = state.user_id;

    // Use first ledger's name if available, otherwise placeholder
    let ledger_name = ledgers
        .first()
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "No Ledger".to_string());

    // Get current user's display name
    let current_user_name = group
        .entities
        .iter()
        .find(|e| e.id == user_uuid)
        .map(|e| e.display_name.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    // Get other group members (excluding current user)
    let group_members: Vec<String> = group
        .entities
        .iter()
        .filter(|e| e.id != user_uuid)
        .map(|e| e.display_name.clone())
        .collect();

    let nav = Header::new()
        .current_ledger(&ledger_name)
        .current_user(&current_user_name)
        .group_members(group_members)
        .build();

    Ok(nav)
}

#[tauri::command]
pub fn render_ledger_header(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;

    // Get current ledger from state
    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;
    let user_uuid = state.user_id;

    // Find the ledger
    let ledger_with_txns = ledgers
        .iter()
        .find(|l| l.ledger.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;

    // Calculate per-user balances from all transactions
    let mut balances: std::collections::HashMap<Uuid, f64> = std::collections::HashMap::new();
    let mut currency = String::from("USD");

    for txn in &ledger_with_txns.transactions {
        // Use the currency from the first transaction
        if currency == "USD" {
            currency = txn.currency_iso_4217.clone();
        }

        // Calculate user's share
        let user_ratio = txn
            .split_ratios
            .iter()
            .find(|s| s.entity_id == user_uuid)
            .map(|s| s.ratio.numerator() as f64 / s.ratio.denominator() as f64)
            .unwrap_or(0.0);

        let user_share = txn.amount * user_ratio;

        // If user paid, they are owed money by others
        if txn.paid_by_entity == user_uuid {
            // User is owed by each person in the split
            for split in &txn.split_ratios {
                if split.entity_id != user_uuid {
                    let other_share = txn.amount * (split.ratio.numerator() as f64 / split.ratio.denominator() as f64);
                    *balances.entry(split.entity_id).or_insert(0.0) += other_share;
                }
            }
        } else if txn.paid_by_entity != user_uuid {
            // User owes money to the person who paid
            *balances.entry(txn.paid_by_entity).or_insert(0.0) -= user_share;
        }
    }

    // Get the group entities to map UUIDs to names
    let group = state.group.lock().map_err(|e| e.to_string())?;
    
    // Convert HashMap to Vec of (name, amount) pairs, filtering out the current user
    let mut balance_list: Vec<(String, f64)> = balances
        .into_iter()
        .filter(|(_, amount)| amount.abs() > 0.01) // Filter out near-zero balances
        .filter_map(|(entity_id, amount)| {
            group
                .entities
                .iter()
                .find(|e| e.id == entity_id)
                .map(|e| (e.display_name.clone(), amount))
        })
        .collect();
    
    // Sort by absolute amount descending
    balance_list.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));

    // Collect all available ledgers for the dropdown
    let available_ledgers: Vec<(String, String)> = ledgers
        .iter()
        .map(|l| (l.ledger.id.to_string(), l.ledger.display_name.clone()))
        .collect();

    let header = LedgerHeader::new()
        .ledger_name(&ledger_with_txns.ledger.display_name)
        .balances(balance_list)
        .currency(&currency)
        .ledgers(available_ledgers)
        .build();

    Ok(header)
}

#[tauri::command]
pub fn switch_ledger(ledger_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    // Parse the ledger_id as UUID and find the matching ledger
    let uuid = Uuid::parse_str(&ledger_id).map_err(|e| e.to_string())?;

    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let user_uuid = state.user_id;

    // Find the ledger with the matching ID
    let ledger_name = ledgers
        .iter()
        .find(|l| l.ledger.id == uuid)
        .map(|l| l.ledger.display_name.clone())
        .unwrap_or_else(|| "Unknown Ledger".to_string());

    // Get current user's display name
    let current_user_name = group
        .entities
        .iter()
        .find(|e| e.id == user_uuid)
        .map(|e| e.display_name.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    // Get other group members (excluding current user)
    let group_members: Vec<String> = group
        .entities
        .iter()
        .filter(|e| e.id != user_uuid)
        .map(|e| e.display_name.clone())
        .collect();

    let nav = Header::new()
        .current_ledger(&ledger_name)
        .current_user(&current_user_name)
        .group_members(group_members)
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
            .expense_id(txn.id.to_string())
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

#[tauri::command]
pub fn get_expense(expense_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    use crate::components::ExpenseForm;
    
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    
    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;
    
    // Parse expense ID
    let expense_uuid = Uuid::parse_str(&expense_id).map_err(|e| e.to_string())?;
    
    // Find the ledger
    let ledger_with_txns = ledgers
        .iter()
        .find(|l| l.ledger.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;
    
    // Find the transaction
    let txn = ledger_with_txns
        .transactions
        .iter()
        .find(|t| t.id == expense_uuid)
        .ok_or_else(|| "Transaction not found".to_string())?;
    
    // Get available participants from the group
    let participants: Vec<(String, String)> = group
        .entities
        .iter()
        .map(|e| (e.id.to_string(), e.display_name.clone()))
        .collect();
    
    // Build the expense form
    let form = ExpenseForm::new()
        .expense_id(expense_id)
        .description(&txn.description)
        .paid_by(&txn.paid_by_entity.to_string())
        .amount(txn.amount)
        .currency(&txn.currency_iso_4217)
        .date(txn.transaction_datetime_rfc_3339.to_string())
        .split_ratios(txn.split_ratios.clone())
        .participants(participants)
        .build();
    
    Ok(form)
}
