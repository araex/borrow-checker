use serde::{Deserialize, Serialize};

/// JSON response structures for the frontend

#[derive(Serialize)]
pub struct AppStateResponse {
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub group_members: Vec<String>,
    pub ledgers: Vec<LedgerInfo>,
    pub current_ledger_id: Option<String>,
    pub balances: Vec<BalanceInfo>,
    pub currency: String,
}

#[derive(Serialize)]
pub struct LedgerInfo {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct BalanceInfo {
    pub user_name: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct TransactionResponse {
    pub expense_id: String,
    pub description: String,
    pub payer_name: String,
    pub total_amount: f64,
    pub currency: String,
    pub date: String,
    pub user_amount: f64,
}

#[derive(Serialize)]
pub struct EntityInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct ExpenseDetailResponse {
    pub id: String,
    pub description: String,
    pub amount: f64,
    pub currency: String,
    pub paid_by: String,
    pub date: String,
    pub split_ratios: Vec<SplitInfo>,
    pub participants: Vec<ParticipantInfo>,
}

#[derive(Serialize)]
pub struct SplitInfo {
    pub entity_id: String,
    pub numerator: i64,
    pub denominator: i64,
}

#[derive(Serialize)]
pub struct ParticipantInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct SplitInput {
    pub entity_id: String,
    pub numerator: i64,
    pub denominator: i64,
}

#[derive(Serialize)]
pub struct SettingsResponse {
    pub user_id: String,
    pub user_name: String,
    pub group_remote_url: String,
    pub group_members: Vec<String>,
    pub ledgers: Vec<LedgerSettingsInfo>,
    pub current_ledger: String,
    pub ssh_private_key_path: String,
    pub ssh_public_key: String,
}

#[derive(Serialize)]
pub struct LedgerSettingsInfo {
    pub id: String,
    pub name: String,
    pub is_current: bool,
}

#[derive(Serialize)]
pub struct RefreshDataResponse {
    pub state_changed: bool,
}
/// Configuration export data structure for QR code sharing
#[derive(Serialize, Deserialize)]
pub struct ConfigExportData {
    pub group_remote_url: String,
    pub private_key: String,
    pub public_key: String,
}

#[derive(Serialize)]
pub struct SettlementPayment {
    pub from_name: String,
    pub to_name: String,
    pub amount: f64,
    pub currency: String,
}

#[derive(Serialize)]
pub struct CurrencyInfo {
    pub code: String,
    pub total_amount: f64,
}

#[derive(Serialize)]
pub struct ConvertedTransaction {
    pub description: String,
    pub amount: f64,
    pub original_currency: String,
    pub converted_amount: f64,
    pub target_currency: String,
    pub conversion_rate: f64,
    pub date: String,
}

#[derive(Serialize)]
pub struct SettlementResponse {
    pub payments: Vec<SettlementPayment>,
    pub currencies: Vec<CurrencyInfo>,
    pub total_converted: Option<f64>,
    pub target_currency: Option<String>,
    pub converted_transactions: Vec<ConvertedTransaction>,
}

#[derive(Deserialize)]
pub struct CurrencyConversionInput {
    pub currency_code: String,
    pub fixed_rate: Option<f64>,
}