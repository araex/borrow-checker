use crate::structs::Split;
/// UI Components for Borrow Checker
///
/// All components use the builder pattern for flexible construction
/// and return HTML strings styled with Tailwind CSS classes.
use askama::Template;

#[derive(Template)]
#[template(path = "header.html")]
pub struct Header {
    current_ledger: Option<String>,
    current_user_name: Option<String>,
    group_members: Vec<String>,
}

impl Header {
    pub fn new() -> Self {
        Self {
            current_ledger: None,
            current_user_name: None,
            group_members: Vec::new(),
        }
    }

    pub fn current_ledger(mut self, ledger_name: impl Into<String>) -> Self {
        self.current_ledger = Some(ledger_name.into());
        self
    }

    pub fn current_user(mut self, user_name: impl Into<String>) -> Self {
        self.current_user_name = Some(user_name.into());
        self
    }

    pub fn group_members(mut self, members: Vec<String>) -> Self {
        self.group_members = members;
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "transaction.html")]
pub struct Transaction {
    expense_id: String,
    description: String,
    payer_name: String,
    total_amount: f64,
    currency: String,
    date: String,
    user_amount: f64,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            expense_id: String::from("unknown"),
            description: String::new(),
            payer_name: String::new(),
            total_amount: 0.0,
            currency: String::from("USD"),
            date: String::new(),
            user_amount: 0.0,
        }
    }

    pub fn expense_id(mut self, id: impl Into<String>) -> Self {
        self.expense_id = id.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn payer_name(mut self, name: impl Into<String>) -> Self {
        self.payer_name = name.into();
        self
    }

    pub fn total_amount(mut self, amount: f64) -> Self {
        self.total_amount = amount;
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = date.into();
        self
    }

    pub fn borrowed(mut self, amount: f64) -> Self {
        self.user_amount = -amount;
        self
    }

    pub fn lent(mut self, amount: f64) -> Self {
        self.user_amount = amount;
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "ledger_header.html")]
pub struct LedgerHeader {
    ledger_name: String,
    balances: Vec<(String, f64)>, // (user_name, amount) pairs
    currency: String,
    ledgers: Vec<(String, String)>, // (id, name) pairs
    ledger_id: String,
}

impl LedgerHeader {
    pub fn new() -> Self {
        Self {
            ledger_name: String::new(),
            balances: Vec::new(),
            currency: String::from("USD"),
            ledgers: Vec::new(),
            ledger_id: String::new(),
        }
    }

    pub fn ledger_name(mut self, name: impl Into<String>) -> Self {
        self.ledger_name = name.into();
        self
    }

    pub fn ledger_id(mut self, ledger_id: impl Into<String>) -> Self {
        self.ledger_id = ledger_id.into();
        self
    }

    pub fn balances(mut self, balances: Vec<(String, f64)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn ledgers(mut self, ledgers: Vec<(String, String)>) -> Self {
        self.ledgers = ledgers;
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for LedgerHeader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "expense_form.html")]
pub struct ExpenseForm {
    expense_id: Option<String>,
    description: String,
    paid_by: String,
    amount: f64,
    currency: String,
    date: String,
    split_ratios: Vec<Split>,
    participants: Vec<(String, String)>, // (id, display_name) pairs
    form_title: String,
    submit_label: String,
    submit_invoke: String,
}

impl ExpenseForm {
    pub fn new() -> Self {
        Self {
            expense_id: None,
            description: String::new(),
            paid_by: String::new(),
            amount: 0.0,
            currency: String::from("USD"),
            date: String::new(),
            split_ratios: Vec::new(),
            participants: Vec::new(),
            form_title: String::from("Add Expense"),
            submit_label: String::from("Create Expense"),
            submit_invoke: String::from("create_expense"),
        }
    }

    pub fn expense_id(mut self, id: impl Into<String>) -> Self {
        self.expense_id = Some(id.into());
        self.form_title = String::from("Edit Expense");
        self.submit_label = String::from("Update Expense");
        self.submit_invoke = String::from("update_expense");
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn paid_by(mut self, paid_by: impl Into<String>) -> Self {
        self.paid_by = paid_by.into();
        self
    }

    pub fn amount(mut self, amount: f64) -> Self {
        self.amount = amount;
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = date.into();
        self
    }

    pub fn split_ratios(mut self, splits: Vec<Split>) -> Self {
        self.split_ratios = splits;
        self
    }

    pub fn participants(mut self, participants: Vec<(String, String)>) -> Self {
        self.participants = participants;
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for ExpenseForm {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct Settings {
    user_name: String,
    user_id: String,
    group_members: Vec<String>,
    ledgers: Vec<String>,
    current_ledger: String,
    ssh_private_key_path: String,
    ssh_public_key: String,
    group_remote_url: String,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            user_name: String::new(),
            user_id: String::new(),
            group_members: Vec::new(),
            ledgers: Vec::new(),
            current_ledger: String::new(),
            ssh_private_key_path: String::new(),
            ssh_public_key: String::new(),
            group_remote_url: String::new(),
        }
    }

    pub fn user_name(mut self, name: impl Into<String>) -> Self {
        self.user_name = name.into();
        self
    }

    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = id.into();
        self
    }

    pub fn group_members(mut self, members: Vec<String>) -> Self {
        self.group_members = members;
        self
    }

    pub fn ledgers(mut self, ledgers: Vec<String>) -> Self {
        self.ledgers = ledgers;
        self
    }

    pub fn current_ledger(mut self, ledger: impl Into<String>) -> Self {
        self.current_ledger = ledger.into();
        self
    }

    pub fn ssh_private_key_path(mut self, path: impl Into<String>) -> Self {
        self.ssh_private_key_path = path.into();
        self
    }

    pub fn ssh_public_key(mut self, key: impl Into<String>) -> Self {
        self.ssh_public_key = key.into();
        self
    }

    pub fn group_remote_url(mut self, url: impl Into<String>) -> Self {
        self.group_remote_url = url.into();
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "main_content.html")]
pub struct MainContent {
    header: String,
    ledger_header: String,
    transactions: String,
}

impl MainContent {
    pub fn new() -> Self {
        Self {
            header: String::new(),
            ledger_header: String::new(),
            transactions: String::new(),
        }
    }

    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = header.into();
        self
    }

    pub fn ledger_header(mut self, header: impl Into<String>) -> Self {
        self.ledger_header = header.into();
        self
    }

    pub fn transactions(mut self, transactions: impl Into<String>) -> Self {
        self.transactions = transactions.into();
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for MainContent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "onboarding.html")]
pub struct OnboardingScreen {
    ssh_public_key: String,
}

impl OnboardingScreen {
    pub fn new() -> Self {
        Self {
            ssh_public_key: String::new(),
        }
    }

    pub fn ssh_public_key(mut self, key: impl Into<String>) -> Self {
        self.ssh_public_key = key.into();
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for OnboardingScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Template)]
#[template(path = "entity_selection.html")]
pub struct EntitySelection {
    entities: Vec<crate::structs::Entity>,
}

impl EntitySelection {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn entities(mut self, entities: Vec<crate::structs::Entity>) -> Self {
        self.entities = entities;
        self
    }

    pub fn build(self) -> String {
        self.render()
            .unwrap_or_else(|e| format!("Template error: {}", e))
    }
}

impl Default for EntitySelection {
    fn default() -> Self {
        Self::new()
    }
}
