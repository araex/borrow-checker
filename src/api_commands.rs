use crate::api_types::*;
use crate::config::save_config;
use crate::git_adapter::GitPersistence;
use crate::repo_manager::RepoManager;
use crate::structs::{AppState, SplitType};
use crate::traits::SharedPersistence;
use std::sync::Arc;
use tauri::Manager;
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
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
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
    let persistence: SharedPersistence = Arc::new(
        GitPersistence::new(repo_path.clone())
            .map_err(|e| format!("Failed to initialize persistence: {}", e))?,
    );

    let group = persistence
        .load_group()
        .map_err(|e| format!("Failed to load group: {}", e))?
        .unwrap_or_else(|| crate::structs::Group {
            entities: Vec::new(),
        });

    let ledgers = persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    if ledgers.is_empty() {
        return Err("Repository has no ledgers".to_string());
    }

    // Use first ledger as default
    let ledger_id = ledgers[0].id;

    let transactions = persistence
        .list_transactions(ledger_id)
        .map_err(|e| format!("Failed to load transactions: {}", e))?;

    // Update config and save to disk
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.group_remote_url = url.clone();
        config.local_repo_path = Some(repo_path.clone());
        save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    }

    // Update AppState with loaded data (but not user_id yet)
    *state.group.lock().map_err(|e| e.to_string())? = Some(group);
    *state.ledgers.lock().map_err(|e| e.to_string())? = Some(ledgers);
    *state.transactions.lock().map_err(|e| e.to_string())? = Some(transactions);
    *state.current_ledger_id.lock().map_err(|e| e.to_string())? = Some(ledger_id);

    // Register persistence with Tauri's state manager
    app_handle.manage(persistence.clone());

    log::info!("Successfully joined group and loaded data");

    Ok(())
}

/// Get list of entities for selection
#[tauri::command]
pub fn get_entities(state: tauri::State<AppState>) -> Result<Vec<EntityInfo>, String> {
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let group_ref = group.as_ref().ok_or("Group not loaded")?;

    Ok(group_ref
        .entities
        .iter()
        .map(|e| EntityInfo {
            id: e.id.to_string(),
            display_name: e.display_name.clone(),
        })
        .collect())
}

/// Select an existing entity as the current user
#[tauri::command]
pub fn select_entity(entity_id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let entity_uuid =
        Uuid::parse_str(&entity_id).map_err(|e| format!("Invalid entity ID: {}", e))?;

    // Verify entity exists in group
    {
        let group = state.group.lock().map_err(|e| e.to_string())?;
        let group_ref = group.as_ref().ok_or("Group not loaded")?;

        if !group_ref.entities.iter().any(|e| e.id == entity_uuid) {
            return Err("Entity not found in group".to_string());
        }
    }

    // Set user_id in state and config
    *state.user_id.lock().map_err(|e| e.to_string())? = Some(entity_uuid);

    // Save to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.user_id = Some(entity_uuid);
        save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    }

    log::info!("User selected entity: {}", entity_uuid);

    Ok(())
}

/// Add a new entity to the group
#[tauri::command]
pub fn add_new_entity(
    display_name: String,
    state: tauri::State<AppState>,
    persistence: tauri::State<SharedPersistence>,
) -> Result<(), String> {
    if display_name.trim().is_empty() {
        return Err("Display name cannot be empty".to_string());
    }

    let new_entity_id = Uuid::new_v4();
    let persistence = persistence.inner();

    // Add entity to group
    {
        let mut group = state.group.lock().map_err(|e| e.to_string())?;
        let group_ref = group.as_mut().ok_or("Group not loaded")?;

        // Check if entity name already exists
        if group_ref
            .entities
            .iter()
            .any(|e| e.display_name == display_name)
        {
            return Err("An entity with this name already exists".to_string());
        }

        group_ref.entities.push(crate::structs::Entity {
            id: new_entity_id,
            display_name: display_name.clone(),
        });

        // Save to persistence
        persistence
            .save_group(&group_ref)
            .map_err(|e| format!("Failed to save group: {}", e))?;

        log::info!(
            "Added new entity '{}' with ID: {}",
            display_name,
            new_entity_id
        );
    }

    // Set user_id to the new entity in state and config
    *state.user_id.lock().map_err(|e| e.to_string())? = Some(new_entity_id);

    // Save to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.user_id = Some(new_entity_id);
        save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    }

    Ok(())
}

/// Get the complete app state
#[tauri::command]
pub fn get_app_state(state: tauri::State<AppState>) -> Result<AppStateResponse, String> {
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    let user_id = state.user_id.lock().map_err(|e| e.to_string())?;

    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let user_uuid = user_id.as_ref().ok_or("No user selected")?;

    // Get current user's display name
    let current_user_name = group_ref
        .entities
        .iter()
        .find(|e| e.id == *user_uuid)
        .map(|e| e.display_name.clone());

    // Get other group members (excluding current user)
    let group_members: Vec<String> = group_ref
        .entities
        .iter()
        .filter(|e| e.id != *user_uuid)
        .map(|e| e.display_name.clone())
        .collect();

    // Get ledgers
    let ledger_list: Vec<LedgerInfo> = ledgers_ref
        .iter()
        .map(|l| LedgerInfo {
            id: l.id.to_string(),
            name: l.display_name.clone(),
        })
        .collect();

    // Calculate balances
    let balances_map = crate::accounting::calculate_balances(&transactions_ref, *user_uuid);
    let currency = crate::accounting::get_primary_currency(&transactions_ref);

    let balance_list: Vec<BalanceInfo> = balances_map
        .into_iter()
        .filter(|(_, amount)| amount.abs() > 0.01)
        .filter_map(|(entity_id, amount)| {
            group_ref
                .entities
                .iter()
                .find(|e| e.id == entity_id)
                .map(|e| BalanceInfo {
                    user_name: e.display_name.clone(),
                    amount,
                })
        })
        .collect();

    Ok(AppStateResponse {
        user_id: Some(user_uuid.to_string()),
        user_name: current_user_name,
        group_members,
        ledgers: ledger_list,
        current_ledger_id: current_ledger_id.map(|id| id.to_string()),
        balances: balance_list,
        currency,
    })
}

/// Get transactions for the current ledger
#[tauri::command]
pub fn get_transactions(state: tauri::State<AppState>) -> Result<Vec<TransactionResponse>, String> {
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let user_id = state.user_id.lock().map_err(|e| e.to_string())?;

    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let user_uuid = user_id.as_ref().ok_or("No user selected")?;

    let result: Vec<TransactionResponse> = transactions_ref
        .iter()
        .map(|t| {
            let payer_name = group_ref
                .entities
                .iter()
                .find(|e| e.id == t.paid_by_entity)
                .map(|e| e.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let user_amount = crate::accounting::calculate_user_amount(t, *user_uuid);

            TransactionResponse {
                expense_id: t.id.to_string(),
                description: t.description.clone(),
                payer_name,
                total_amount: t.amount,
                currency: t.currency_iso_4217.clone(),
                date: t.transaction_datetime_rfc_3339.to_string(),
                user_amount,
            }
        })
        .collect();

    Ok(result)
}

/// Switch to a different ledger
#[tauri::command]
pub fn switch_ledger(
    ledger_id: String,
    state: tauri::State<AppState>,
    persistence: tauri::State<SharedPersistence>,
) -> Result<(), String> {
    let ledger_uuid =
        Uuid::parse_str(&ledger_id).map_err(|e| format!("Invalid ledger ID: {}", e))?;

    let persistence = persistence.inner();

    // Build ledger map by listing ledgers
    persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    let transactions = persistence
        .list_transactions(ledger_uuid)
        .map_err(|e| format!("Failed to load transactions: {}", e))?;

    *state.current_ledger_id.lock().map_err(|e| e.to_string())? = Some(ledger_uuid);
    *state.transactions.lock().map_err(|e| e.to_string())? = Some(transactions);

    Ok(())
}

/// Get detailed expense information for editing
#[tauri::command]
pub fn get_expense(
    expense_id: String,
    state: tauri::State<AppState>,
) -> Result<ExpenseDetailResponse, String> {
    let expense_uuid =
        Uuid::parse_str(&expense_id).map_err(|e| format!("Invalid expense ID: {}", e))?;

    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;

    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;

    let transaction = transactions_ref
        .iter()
        .find(|t| t.id == expense_uuid)
        .ok_or("Expense not found")?;

    let split_ratios: Vec<SplitInfo> = transaction
        .split_ratios
        .iter()
        .map(|split| SplitInfo {
            entity_id: split.entity_id.to_string(),
            numerator: split.ratio.numerator() as i64,
            denominator: split.ratio.denominator() as i64,
        })
        .collect();

    let participants: Vec<ParticipantInfo> = group_ref
        .entities
        .iter()
        .map(|e| ParticipantInfo {
            id: e.id.to_string(),
            display_name: e.display_name.clone(),
        })
        .collect();

    Ok(ExpenseDetailResponse {
        id: transaction.id.to_string(),
        description: transaction.description.clone(),
        amount: transaction.amount,
        currency: transaction.currency_iso_4217.clone(),
        paid_by: transaction.paid_by_entity.to_string(),
        date: transaction.transaction_datetime_rfc_3339.to_string(),
        split_ratios,
        participants,
    })
}

/// Create a new expense
#[tauri::command]
pub fn create_expense(
    description: String,
    amount: f64,
    currency: String,
    paid_by: String,
    date: String,
    split_ratios: Vec<SplitInput>,
    state: tauri::State<AppState>,
    persistence: tauri::State<SharedPersistence>,
) -> Result<String, String> {
    use rational::Rational;
    use toml::value::Datetime;

    let paid_by_uuid =
        Uuid::parse_str(&paid_by).map_err(|e| format!("Invalid paid_by ID: {}", e))?;
    let expense_id = Uuid::new_v4();

    // Parse date string to Datetime
    let datetime: Datetime = date
        .parse()
        .map_err(|e| format!("Invalid date format: {}", e))?;

    // Convert split inputs to Split structs
    let splits: Vec<crate::structs::Split> = split_ratios
        .into_iter()
        .map(|input| {
            let entity_id = Uuid::parse_str(&input.entity_id)
                .map_err(|e| format!("Invalid entity ID: {}", e))?;
            Ok(crate::structs::Split {
                entity_id,
                ratio: Rational::new(input.numerator, input.denominator),
                split_type: SplitType::Ratio(Rational::new(input.numerator, input.denominator)),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let transaction = crate::structs::Transaction {
        id: expense_id,
        description,
        paid_by_entity: paid_by_uuid,
        currency_iso_4217: currency,
        amount,
        transaction_datetime_rfc_3339: datetime,
        split_ratios: splits,
    };

    // Save to persistence
    let current_ledger_id = state
        .current_ledger_id
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or("No ledger selected")?
        .clone();

    let persistence = persistence.inner();

    // Build ledger map by listing ledgers
    persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    persistence
        .create_transaction(current_ledger_id, transaction.clone())
        .map_err(|e| format!("Failed to save transaction: {}", e))?;

    // Update state
    let mut transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut txns) = *transactions {
        txns.push(transaction);
    }

    log::info!("Created expense: {}", expense_id);
    Ok(expense_id.to_string())
}

/// Update an existing expense
#[tauri::command]
pub fn update_expense(
    expense_id: String,
    description: String,
    amount: f64,
    currency: String,
    paid_by: String,
    date: String,
    split_ratios: Vec<SplitInput>,
    state: tauri::State<AppState>,
    persistence: tauri::State<SharedPersistence>,
) -> Result<(), String> {
    use rational::Rational;
    use toml::value::Datetime;

    let expense_uuid =
        Uuid::parse_str(&expense_id).map_err(|e| format!("Invalid expense ID: {}", e))?;
    let paid_by_uuid =
        Uuid::parse_str(&paid_by).map_err(|e| format!("Invalid paid_by ID: {}", e))?;

    let datetime: Datetime = date
        .parse()
        .map_err(|e| format!("Invalid date format: {}", e))?;

    let splits: Vec<crate::structs::Split> = split_ratios
        .into_iter()
        .map(|input| {
            let entity_id = Uuid::parse_str(&input.entity_id)
                .map_err(|e| format!("Invalid entity ID: {}", e))?;
            Ok(crate::structs::Split {
                entity_id,
                ratio: Rational::new(input.numerator, input.denominator),
                split_type: SplitType::Ratio(Rational::new(input.numerator, input.denominator)),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let transaction = crate::structs::Transaction {
        id: expense_uuid,
        description,
        paid_by_entity: paid_by_uuid,
        currency_iso_4217: currency,
        amount,
        transaction_datetime_rfc_3339: datetime,
        split_ratios: splits,
    };

    // Save to persistence
    let current_ledger_id = state
        .current_ledger_id
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or("No ledger selected")?
        .clone();

    let persistence = persistence.inner();

    // Build ledger map by listing ledgers
    persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    persistence
        .update_transaction(current_ledger_id, transaction.clone())
        .map_err(|e| format!("Failed to update transaction: {}", e))?;

    // Update state
    let mut transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut txns) = *transactions {
        if let Some(pos) = txns.iter().position(|t| t.id == expense_uuid) {
            txns[pos] = transaction;
        }
    }

    log::info!("Updated expense: {}", expense_id);
    Ok(())
}

/// Delete an expense
#[tauri::command]
pub fn delete_expense(
    expense_id: String,
    state: tauri::State<AppState>,
    persistence: tauri::State<SharedPersistence>,
) -> Result<(), String> {
    let expense_uuid =
        Uuid::parse_str(&expense_id).map_err(|e| format!("Invalid expense ID: {}", e))?;

    // Delete from persistence
    let current_ledger_id = state
        .current_ledger_id
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or("No ledger selected")?
        .clone();

    let persistence = persistence.inner();

    // Build ledger map by listing ledgers
    persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    persistence
        .delete_transaction(current_ledger_id, expense_uuid)
        .map_err(|e| format!("Failed to delete transaction: {}", e))?;

    // Update state
    let mut transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut txns) = *transactions {
        txns.retain(|t| t.id != expense_uuid);
    }

    log::info!("Deleted expense: {}", expense_id);
    Ok(())
}

/// Get settings data
#[tauri::command]
pub fn get_settings(state: tauri::State<AppState>) -> Result<SettingsResponse, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    let ledgers = state.ledgers.lock().map_err(|e| e.to_string())?;
    let current_ledger_id = state.current_ledger_id.lock().map_err(|e| e.to_string())?;
    let user_id = state.user_id.lock().map_err(|e| e.to_string())?;

    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    let ledgers_ref = ledgers.as_ref().ok_or("Not onboarded")?;
    let user_uuid = user_id.as_ref().ok_or("No user selected")?;
    let current_ledger_uuid = current_ledger_id.ok_or("No ledger selected")?;

    let user_name = group_ref
        .entities
        .iter()
        .find(|e| e.id == *user_uuid)
        .map(|e| e.display_name.clone())
        .ok_or("User not found")?;

    let group_members: Vec<String> = group_ref
        .entities
        .iter()
        .map(|e| e.display_name.clone())
        .collect();

    let current_ledger_name = ledgers_ref
        .iter()
        .find(|l| l.id == current_ledger_uuid)
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let ledger_list: Vec<LedgerSettingsInfo> = ledgers_ref
        .iter()
        .map(|l| LedgerSettingsInfo {
            id: l.id.to_string(),
            name: l.display_name.clone(),
            is_current: l.id == current_ledger_uuid,
        })
        .collect();

    let ssh_private_key_path = crate::ssh_keys::get_private_key_path()
        .map_err(|e| format!("Failed to get SSH key path: {}", e))?
        .to_string_lossy()
        .to_string();

    let ssh_public_key = crate::ssh_keys::get_public_key_content()
        .map_err(|e| format!("Failed to read SSH public key: {}", e))?;

    Ok(SettingsResponse {
        user_id: user_uuid.to_string(),
        user_name,
        group_remote_url: config.group_remote_url.clone(),
        group_members,
        ledgers: ledger_list,
        current_ledger: current_ledger_name,
        ssh_private_key_path,
        ssh_public_key,
    })
}

/// Refresh data from remote repository
#[tauri::command]
pub async fn refresh_data(
    persistence: tauri::State<'_, SharedPersistence>,
) -> Result<RefreshDataResponse, String> {
    let persistence = persistence.inner().clone();
    log::info!("Start Data refresh",);

    let result = tauri::async_runtime::spawn_blocking(move || persistence.refresh())
        .await
        .map_err(|e| format!("Failed to join refresh task: {}", e))?
        .map_err(|e| format!("Failed to refresh data: {}", e))?;

    log::info!(
        "Data refresh completed, remote_changed: {}, pushed: {}",
        result.remote_changed,
        result.pushed
    );

    Ok(RefreshDataResponse {
        state_changed: result.remote_changed,
        remote_changed: result.remote_changed,
        pushed: result.pushed,
    })
}

/// Reset user selection (return to entity selection screen)
#[tauri::command]
pub fn reset_user(state: tauri::State<AppState>) -> Result<(), String> {
    *state.user_id.lock().map_err(|e| e.to_string())? = None;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.user_id = None;
    save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("User reset, returning to entity selection");
    Ok(())
}
/// Export configuration and SSH keys as a QR-code compatible string
#[tauri::command]
pub fn export_config_qr(state: tauri::State<AppState>) -> Result<String, String> {
    use qrcode::QrCode;
    use qrcode::render::svg;

    // Load config
    let config = state.config.lock().map_err(|e| e.to_string())?;

    // Read SSH keys
    let private_key = crate::ssh_keys::get_private_key_content()
        .map_err(|e| format!("Failed to read private key: {}", e))?;
    let public_key = crate::ssh_keys::get_public_key_content()
        .map_err(|e| format!("Failed to read public key: {}", e))?;

    // Create export data structure
    let export_data = ConfigExportData {
        group_remote_url: config.group_remote_url.clone(),
        private_key,
        public_key,
    };

    // Serialize to JSON
    let json_data = serde_json::to_string(&export_data)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Generate QR code as SVG
    let code = QrCode::new(json_data.as_bytes())
        .map_err(|e| format!("Failed to generate QR code: {}", e))?;

    let svg_string = code.render::<svg::Color>().min_dimensions(400, 400).build();

    Ok(svg_string)
}

/// Import configuration and SSH keys from QR-code data
#[tauri::command]
pub async fn import_config_qr(
    qr_data: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Parse QR data
    let export_data: ConfigExportData =
        serde_json::from_str(&qr_data).map_err(|e| format!("Invalid QR code data: {}", e))?;

    // Import SSH keys
    crate::ssh_keys::import_ssh_keys(&export_data.private_key, &export_data.public_key)
        .map_err(|e| format!("Failed to import SSH keys: {}", e))?;

    // Clone the repository (reuse existing join_group logic)
    let url_clone = export_data.group_remote_url.clone();
    let repo_path =
        tauri::async_runtime::spawn_blocking(move || RepoManager::clone_repository(&url_clone))
            .await
            .map_err(|e| format!("Task join error: {}", e))??;

    // Validate repository structure
    RepoManager::validate_repo_structure(&repo_path)?;

    // Load data from the repository
    let persistence: SharedPersistence = Arc::new(
        GitPersistence::new(repo_path.clone())
            .map_err(|e| format!("Failed to initialize persistence: {}", e))?,
    );

    let group = persistence
        .load_group()
        .map_err(|e| format!("Failed to load group: {}", e))?
        .ok_or_else(|| "Group configuration not found. Initialize the repository first.".to_string())?;

    let ledgers = persistence
        .list_ledgers()
        .map_err(|e| format!("Failed to load ledgers: {}", e))?;

    if ledgers.is_empty() {
        return Err("Repository has no ledgers".to_string());
    }

    // Use first ledger as default
    let ledger_id = ledgers[0].id;

    let transactions = persistence
        .list_transactions(ledger_id)
        .map_err(|e| format!("Failed to load transactions: {}", e))?;

    // Update config and save to disk
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.group_remote_url = export_data.group_remote_url.clone();
        config.local_repo_path = Some(repo_path.clone());
        save_config(&config).map_err(|e| format!("Failed to save config: {}", e))?;
    }

    // Update AppState with loaded data (but not user_id yet)
    *state.group.lock().map_err(|e| e.to_string())? = Some(group);
    *state.ledgers.lock().map_err(|e| e.to_string())? = Some(ledgers);
    *state.transactions.lock().map_err(|e| e.to_string())? = Some(transactions);
    *state.current_ledger_id.lock().map_err(|e| e.to_string())? = Some(ledger_id);

    // Register persistence with Tauri's state manager
    app_handle.manage(persistence.clone());

    log::info!("Successfully imported config from QR code");

    Ok(())
}

/// Get settlement information - who owes who with minimal payments
#[tauri::command]
pub fn get_settlement(state: tauri::State<AppState>) -> Result<SettlementResponse, String> {
    let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
    let group = state.group.lock().map_err(|e| e.to_string())?;
    
    let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
    let group_ref = group.as_ref().ok_or("Not onboarded")?;
    
    // Build entity name map
    let entity_names: std::collections::HashMap<Uuid, String> = group_ref
        .entities
        .iter()
        .map(|e| (e.id, e.display_name.clone()))
        .collect();
    
    // Calculate optimal settlement payments
    let payments = crate::accounting::calculate_settlement_payments(transactions_ref, &entity_names);
    
    let settlement_payments: Vec<SettlementPayment> = payments
        .into_iter()
        .map(|(from, to, amount, currency)| SettlementPayment {
            from_name: from,
            to_name: to,
            amount,
            currency,
        })
        .collect();
    
    // Get all currencies
    let currency_totals = crate::accounting::get_all_currencies(transactions_ref);
    let currencies: Vec<CurrencyInfo> = currency_totals
        .into_iter()
        .map(|(code, total_amount)| CurrencyInfo {
            code,
            total_amount,
        })
        .collect();
    
    Ok(SettlementResponse {
        payments: settlement_payments,
        currencies,
        total_converted: None,
        target_currency: None,
        converted_transactions: Vec::new(),
    })
}

/// Convert settlement to a target currency with conversion rates
#[tauri::command]
pub async fn convert_settlement(
    target_currency: String,
    conversion_rates: Vec<CurrencyConversionInput>,
    state: tauri::State<'_, AppState>,
) -> Result<SettlementResponse, String> {
    use std::collections::HashMap;
    
    // Clone the data we need before any async operations to avoid holding locks across await points
    let (transactions_clone, entity_names) = {
        let transactions = state.transactions.lock().map_err(|e| e.to_string())?;
        let group = state.group.lock().map_err(|e| e.to_string())?;
        
        let transactions_ref = transactions.as_ref().ok_or("Not onboarded")?;
        let group_ref = group.as_ref().ok_or("Not onboarded")?;
        
        let entity_names: HashMap<Uuid, String> = group_ref
            .entities
            .iter()
            .map(|e| (e.id, e.display_name.clone()))
            .collect();
        
        (transactions_ref.clone(), entity_names)
    };
    
    // Build conversion rate map
    let mut rate_map: HashMap<String, f64> = HashMap::new();
    for rate_input in conversion_rates {
        if let Some(fixed_rate) = rate_input.fixed_rate {
            rate_map.insert(rate_input.currency_code.clone(), fixed_rate);
        }
    }
    
    // For currencies without fixed rates, fetch from API
    let all_currencies = crate::accounting::get_all_currencies(&transactions_clone);
    for (currency, _) in &all_currencies {
        if currency != &target_currency && !rate_map.contains_key(currency) {
            // Try to fetch rate from API
            let rate = fetch_conversion_rate(currency, &target_currency).await
                .unwrap_or(1.0); // Fallback to 1.0 if fetch fails
            rate_map.insert(currency.clone(), rate);
        }
    }
    
    // Convert all transactions to target currency
    let mut converted_transactions = Vec::new();
    let mut converted_amounts: HashMap<Uuid, (f64, String)> = HashMap::new();
    
    for transaction in &transactions_clone {
        let rate = if transaction.currency_iso_4217 == target_currency {
            1.0
        } else {
            *rate_map.get(&transaction.currency_iso_4217).unwrap_or(&1.0)
        };
        
        let converted_amount = transaction.amount * rate;
        
        converted_transactions.push(ConvertedTransaction {
            description: transaction.description.clone(),
            amount: transaction.amount,
            original_currency: transaction.currency_iso_4217.clone(),
            converted_amount,
            target_currency: target_currency.clone(),
            conversion_rate: rate,
            date: transaction.transaction_datetime_rfc_3339.to_string(),
        });
        
        // Track converted balances
        let paid_by = transaction.paid_by_entity;
        for split in &transaction.split_ratios {
            let entity_id = split.entity_id;
            let ratio = split.ratio.decimal_value();
            let share = converted_amount * ratio;
            
            if entity_id == paid_by {
                let net = converted_amount - share;
                let entry = converted_amounts.entry(entity_id).or_insert((0.0, target_currency.clone()));
                entry.0 += net;
            } else {
                let entry = converted_amounts.entry(entity_id).or_insert((0.0, target_currency.clone()));
                entry.0 -= share;
                
                let payer_entry = converted_amounts.entry(paid_by).or_insert((0.0, target_currency.clone()));
                payer_entry.0 += share;
            }
        }
    }
    
    // Calculate settlement payments with converted amounts
    let mut creditors: Vec<(Uuid, f64)> = Vec::new();
    let mut debtors: Vec<(Uuid, f64)> = Vec::new();
    let mut total_converted = 0.0;
    
    for (entity_id, (balance, _)) in converted_amounts {
        total_converted += balance.abs();
        if balance > 0.01 {
            creditors.push((entity_id, balance));
        } else if balance < -0.01 {
            debtors.push((entity_id, -balance));
        }
    }
    
    let mut payments = Vec::new();
    let mut creditor_idx = 0;
    let mut debtor_idx = 0;
    
    while creditor_idx < creditors.len() && debtor_idx < debtors.len() {
        let (creditor_id, mut creditor_amount) = creditors[creditor_idx];
        let (debtor_id, mut debtor_amount) = debtors[debtor_idx];
        
        let payment_amount = creditor_amount.min(debtor_amount);
        
        let from_name = entity_names.get(&debtor_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let to_name = entity_names.get(&creditor_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        
        payments.push(SettlementPayment {
            from_name,
            to_name,
            amount: payment_amount,
            currency: target_currency.clone(),
        });
        
        creditor_amount -= payment_amount;
        debtor_amount -= payment_amount;
        
        if creditor_amount < 0.01 {
            creditor_idx += 1;
        } else {
            creditors[creditor_idx].1 = creditor_amount;
        }
        
        if debtor_amount < 0.01 {
            debtor_idx += 1;
        } else {
            debtors[debtor_idx].1 = debtor_amount;
        }
    }
    
    let currencies: Vec<CurrencyInfo> = all_currencies
        .into_iter()
        .map(|(code, total_amount)| CurrencyInfo {
            code,
            total_amount,
        })
        .collect();
    
    Ok(SettlementResponse {
        payments,
        currencies,
        total_converted: Some(total_converted / 2.0), // Divide by 2 because we counted both sides
        target_currency: Some(target_currency),
        converted_transactions,
    })
}

/// Fetch currency conversion rate from an external API
/// This is a simplified version - in production, use a proper API like exchangerate-api.com
async fn fetch_conversion_rate(from_currency: &str, to_currency: &str) -> Result<f64, String> {
    // Using exchangerate-api.com free tier (no auth needed for basic usage)
    let url = format!(
        "https://api.exchangerate-api.com/v4/latest/{}",
        from_currency
    );
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch exchange rate: {}", e))?;
    
    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse exchange rate response: {}", e))?;
    
    let rate = data["rates"][to_currency]
        .as_f64()
        .ok_or_else(|| format!("Rate not found for {}", to_currency))?;
    
    Ok(rate)
}