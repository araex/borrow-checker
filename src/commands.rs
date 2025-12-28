use crate::components::{Header, LedgerHeader, MainContent, Settings, Transaction};
use crate::config::save_config;
use crate::git_adapter::GitPersistence;
use crate::repo_manager::RepoManager;
use crate::structs::AppState;
use crate::traits::PersistenceRepository;
use uuid::Uuid;

/// Get the SSH public key for display during onboarding
#[tauri::command]
pub fn get_ssh_public_key() -> Result<String, String> {
    crate::ssh_keys::get_public_key_content()
        .map(|key| key.trim().to_string())
        .map_err(|e| format!("Failed to read SSH public key: {}", e))
}

/// Check if the user has completed onboarding
#[tauri::command]
pub fn is_onboarded(state: tauri::State<AppState>) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.local_repo_path.is_some())
}

/// Join a group by cloning the repository and validating its structure
#[tauri::command]
pub async fn join_group(
    url: String,
    _app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    log::info!("Attempting to join group with URL: {}", url);

    // Clone the repository in a blocking thread to avoid blocking the UI
    let url_clone = url.clone();
    let repo_path =
        tauri::async_runtime::spawn_blocking(move || RepoManager::clone_repository(&url_clone))
            .await
            .map_err(|e| format!("Task join error: {}", e))??;

    // Validate repository structure
    RepoManager::validate_repo_structure(&repo_path)?;

    // Load data from the repository
    let persistence = GitPersistence::new(repo_path.clone())
        .map_err(|e| format!("Failed to initialize persistence: {}", e))?;

    let group = persistence
        .load_group()
        .map_err(|e| format!("Failed to load group: {}", e))?;

    let ledgers = persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    if ledgers.is_empty() {
        return Err("Repository has no ledgers".to_string());
    }

    // Use first ledger and first entity as defaults
    let ledger_id = ledgers[0].id;

    let transactions = persistence
        .list_transactions(ledger_id)
        .map_err(|e| format!("Failed to load transactions: {}", e))?;

    // Update config and save to disk
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.group_remote_url = url.clone();
        config.local_repo_path = Some(repo_path);
        save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    }

    // Update AppState with loaded data
    let user_id = group.entities.get(0).ok_or("Group has no entities")?.id;
    *state.group.lock().map_err(|e| e.to_string())? = Some(group);
    *state.ledgers.lock().map_err(|e| e.to_string())? = Some(ledgers);
    *state.transactions.lock().map_err(|e| e.to_string())? = Some(transactions);
    *state.current_ledger_id.lock().map_err(|e| e.to_string())? = Some(ledger_id);
    *state.user_id.lock().map_err(|e| e.to_string())? = Some(user_id);

    log::info!("Successfully joined group and loaded data");

    // Return the main content to trigger navigation
    render_main_content(state)
}

#[tauri::command]
pub fn render_header(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let user_uuid = *state
        .user_id
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("Not onboarded")?;

    // Get the current ledger's name based on current_ledger_id
    let current_ledger_id = *state
        .current_ledger_id
        .lock()
        .map_err(|e| e.to_string())?;
    
    let ledger_name = current_ledger_id
        .and_then(|id| ledgers_ref.iter().find(|l| l.id == id))
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "No Ledger".to_string());

    // Get current user's display name
    let current_user_name = group_ref
        .entities
        .iter()
        .find(|e| e.id == user_uuid)
        .map(|e| e.display_name.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    // Get other group members (excluding current user)
    let group_members: Vec<String> = group_ref
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
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let user_uuid = *state
        .user_id
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("Not onboarded")?;

    // Get current ledger from state
    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;

    // Find the ledger
    let ledger = ledgers_ref
        .iter()
        .find(|l| l.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;

    // Calculate per-user balances from all transactions
    let balances = crate::accounting::calculate_balances(&transactions_ref, user_uuid);
    let currency = crate::accounting::get_primary_currency(&transactions_ref);

    // Get the group entities to map UUIDs to names
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;

    // Convert HashMap to Vec of (name, amount) pairs, filtering out the current user
    let mut balance_list: Vec<(String, f64)> = balances
        .into_iter()
        .filter(|(_, amount): &(Uuid, f64)| amount.abs() > 0.01) // Filter out near-zero balances
        .filter_map(|(entity_id, amount)| {
            group_ref
                .entities
                .iter()
                .find(|e| e.id == entity_id)
                .map(|e| (e.display_name.clone(), amount))
        })
        .collect();

    // Sort by absolute amount descending
    balance_list.sort_by(|a, b| {
        b.1.abs()
            .partial_cmp(&a.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Collect all available ledgers for the dropdown
    let available_ledgers: Vec<(String, String)> = ledgers_ref
        .iter()
        .map(|l| (l.id.to_string(), l.display_name.clone()))
        .collect();

    let header = LedgerHeader::new()
        .ledger_name(&ledger.display_name)
        .ledger_id(ledger_uuid.to_string())
        .balances(balance_list)
        .currency(&currency)
        .ledgers(available_ledgers)
        .build();

    Ok(header)
}

#[tauri::command]
pub fn switch_ledger(
    ledger_id: String,
    state: tauri::State<AppState>,
    persistence: tauri::State<GitPersistence>,
) -> Result<String, String> {
    // Parse the ledger_id as UUID and find the matching ledger
    let uuid = Uuid::parse_str(&ledger_id).map_err(|e| e.to_string())?;

    let _ = persistence.list_ledgers();
    
    // Update transactions for the new ledger
    {
        let mut transactions_opt = state.transactions.lock().map_err(|e| e.to_string())?;
        *transactions_opt = Some(
            persistence
                .list_transactions(uuid)
                .map_err(|e| e.to_string())?,
        );
    }

    // Update current ledger ID
    {
        let mut guard = state
            .current_ledger_id
            .lock()
            .map_err(|e| format!("mutex poisoned: {e}"))?;
        *guard = Some(uuid);
    }

    // Render the full main content with the new ledger's transactions
    let header = render_header(state.clone())?;
    let ledger_header = render_ledger_header(state.clone())?;
    let transactions = render_transactions(state.clone())?;

    let content = MainContent::new()
        .header(header)
        .ledger_header(ledger_header)
        .transactions(transactions)
        .build();

    Ok(content)
}

#[tauri::command]
pub fn render_transactions(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let user_uuid = *state
        .user_id
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("Not onboarded")?;

    // Get current ledger and user from state
    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;

    // Find the ledger (just for validation)
    let _ledger_with_txns = ledgers_ref
        .iter()
        .find(|l| l.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;

    let mut html = String::from(r#"<section id="expense-list" class="flex flex-col">"#);

    // Render each transaction
    for txn in transactions_ref.iter() {
        // Find the payer's name
        let payer_name = group_ref
            .entities
            .iter()
            .find(|e| e.id == txn.paid_by_entity)
            .map(|e| e.display_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate user's share
        let user_share = crate::accounting::get_user_share(txn, user_uuid);

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

    // Add floating action button
    html.push_str(r###"
        <button
            class="fixed bottom-12 right-12 px-6 py-3 bg-orange-500 hover:bg-orange-600 text-white rounded-full shadow-lg hover:shadow-xl transition-all duration-200 flex items-center gap-2 z-20 font-medium"
            type="button"
            hx-tauri-invoke="show_add_expense_form"
            hx-target="#expense-list"
            title="Add Expense"
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4" />
            </svg>
            <span>Add Expense</span>
        </button>
    "###);

    html.push_str("</section>");

    Ok(html)
}

#[tauri::command]
pub fn get_expense(expense_id: String, state: tauri::State<AppState>) -> Result<String, String> {
    use crate::components::ExpenseForm;

    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;

    let ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;

    // Parse expense ID
    let expense_uuid = Uuid::parse_str(&expense_id).map_err(|e| e.to_string())?;

    // Find the ledger (just for validation)
    let _ledger = ledgers_ref
        .iter()
        .find(|l| l.id == ledger_uuid)
        .ok_or_else(|| "Selected ledger not found".to_string())?;

    // Find the transaction
    let txn = transactions_ref
        .iter()
        .find(|t| t.id == expense_uuid)
        .ok_or_else(|| "Transaction not found".to_string())?;

    // Get available participants from the group
    let participants: Vec<(String, String)> = group_ref
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

#[tauri::command]
pub fn show_add_expense_form(state: tauri::State<AppState>) -> Result<String, String> {
    use crate::components::ExpenseForm;
    use crate::structs::Split;
    use rational::Rational;

    let group = state.group.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let _ledger_uuid = current_ledger_id.ok_or_else(|| "No ledger selected".to_string())?;

    // Get available participants from the group
    let participants: Vec<(String, String)> = group_ref
        .entities
        .iter()
        .map(|e| (e.id.to_string(), e.display_name.clone()))
        .collect();

    // Create default equal split ratios for all participants
    let num_participants = participants.len() as i64;
    let default_split_ratios: Vec<Split> = group_ref
        .entities
        .iter()
        .map(|e| Split {
            entity_id: e.id,
            ratio: Some(Rational::new(1, num_participants)),
            amount: None
        })
        .collect();

    // Build the expense form with defaults for a new expense (date will be empty, browser will default to today)
    let form = ExpenseForm::new()
        .participants(participants)
        .split_ratios(default_split_ratios)
        .build();

    Ok(form)
}

#[tauri::command]
pub fn render_settings(state: tauri::State<AppState>) -> Result<String, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;

    // Handle case where not onboarded
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let user_uuid = *state
        .user_id
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("Not onboarded")?;

    // Get current user's display name
    let user_name = group_ref
        .entities
        .iter()
        .find(|e| e.id == user_uuid)
        .map(|e| e.display_name.clone())
        .unwrap_or_else(|| "Unknown User".to_string());

    // Get all group members
    let group_members: Vec<String> = group_ref
        .entities
        .iter()
        .map(|e| e.display_name.clone())
        .collect();

    // Get all ledger names
    let ledger_names: Vec<String> = ledgers_ref.iter().map(|l| l.display_name.clone()).collect();

    // Get current ledger name
    let current_ledger = current_ledger_id
        .and_then(|id| ledgers_ref.iter().find(|l| l.id == id))
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Unknown Ledger".to_string());

    // Get SSH key information
    let ssh_private_key_path = crate::ssh_keys::get_private_key_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unable to get key path".to_string());

    let ssh_public_key = crate::ssh_keys::get_public_key_content()
        .unwrap_or_else(|_| "Unable to read public key".to_string())
        .trim()
        .to_string();

    let settings = Settings::new()
        .user_name(user_name)
        .user_id(user_uuid.to_string())
        .group_members(group_members)
        .ledgers(ledger_names)
        .current_ledger(current_ledger)
        .ssh_private_key_path(ssh_private_key_path)
        .ssh_public_key(ssh_public_key)
        .group_remote_url(config.group_remote_url.clone())
        .build();

    Ok(settings)
}

#[tauri::command]
pub fn render_main_content(state: tauri::State<AppState>) -> Result<String, String> {
    log::info!("render_main_content called");
    use crate::components::OnboardingScreen;

    // Check if user is onboarded
    let is_onboarded = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.local_repo_path.is_some()
    };

    log::info!("User onboarded status: {}", is_onboarded);

    if !is_onboarded {
        log::info!("Rendering onboarding screen");
        // Show onboarding screen
        let ssh_key = crate::ssh_keys::get_public_key_content()
            .unwrap_or_else(|_| "Unable to read public key".to_string())
            .trim()
            .to_string();

        let onboarding = OnboardingScreen::new().ssh_public_key(ssh_key).build();

        return Ok(onboarding);
    }

    // Render header, ledger header and transactions
    let header = render_header(state.clone())?;
    let ledger_header = render_ledger_header(state.clone())?;
    let transactions = render_transactions(state)?;

    let content = MainContent::new()
        .header(header)
        .ledger_header(ledger_header)
        .transactions(transactions)
        .build();

    Ok(content)
}
