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
    total_amount_formatted: String,
    currency: String,
    date: String,
    status_label: String,
    status_color: String,
    user_amount: f64,
    amount_display: String,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            expense_id: String::from("unknown"),
            description: String::new(),
            payer_name: String::new(),
            total_amount: 0.0,
            total_amount_formatted: String::from("0.00"),
            currency: String::from("USD"),
            date: String::new(),
            status_label: String::new(),
            status_color: String::new(),
            user_amount: 0.0,
            amount_display: String::new(),
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
        self.total_amount_formatted = format!("{:.2}", amount);
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
        self.status_label = String::from("YOU BORROWED");
        self.status_color = String::from("text-red-500");
        self.user_amount = -amount;
        self.amount_display = format!("-{} {:.2}", self.currency, amount.abs());
        self
    }

    pub fn lent(mut self, amount: f64) -> Self {
        self.status_label = String::from("YOU LENT");
        self.status_color = String::from("text-green-400");
        self.user_amount = amount;
        self.amount_display = format!("{} {:.2}", self.currency, amount);
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
    balances: Vec<(String, f64)>,          // (user_name, amount) pairs
    balance_displays: Vec<BalanceDisplay>, // precomputed display data
    currency: String,
    ledgers: Vec<(String, String)>,    // (id, name) pairs
    ledger_options: Vec<LedgerOption>, // precomputed options with selected state
}

#[derive(Debug)]
pub struct BalanceDisplay {
    pub user_name: String,
    pub amount: f64,
    pub amount_formatted: String,
    pub is_negative: bool,
    pub is_positive: bool,
}

#[derive(Debug)]
pub struct LedgerOption {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

impl LedgerHeader {
    pub fn new() -> Self {
        Self {
            ledger_name: String::new(),
            balances: Vec::new(),
            balance_displays: Vec::new(),
            currency: String::from("USD"),
            ledgers: Vec::new(),
            ledger_options: Vec::new(),
        }
    }

    pub fn ledger_name(mut self, name: impl Into<String>) -> Self {
        self.ledger_name = name.into();
        self
    }

    pub fn balances(mut self, balances: Vec<(String, f64)>) -> Self {
        // Precompute balance display data
        self.balance_displays = balances
            .iter()
            .map(|(user_name, amount)| BalanceDisplay {
                user_name: user_name.clone(),
                amount: *amount,
                amount_formatted: format!("{:.2}", amount.abs()),
                is_negative: *amount < 0.0,
                is_positive: *amount > 0.0,
            })
            .collect();

        self.balances = balances;
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn ledgers(mut self, ledgers: Vec<(String, String)>) -> Self {
        // Precompute ledger options with selected state
        self.ledger_options = ledgers
            .iter()
            .map(|(id, name)| LedgerOption {
                id: id.clone(),
                name: name.clone(),
                selected: name == &self.ledger_name,
            })
            .collect();

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
    amount_formatted: String,
    currency: String,
    date: String,
    date_only: String,
    split_ratios: Vec<Split>,
    participants: Vec<(String, String)>, // (id, display_name) pairs
    participant_splits: Vec<ParticipantSplit>, // precomputed split data
    participant_options: Vec<ParticipantOption>, // precomputed participant options
    form_title: String,
    submit_label: String,
    submit_invoke: String,
}

#[derive(Debug)]
pub struct ParticipantSplit {
    pub id: String,
    pub name: String,
    pub is_included: bool,
    pub ratio_value: String,
}

#[derive(Debug)]
pub struct ParticipantOption {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

impl ExpenseForm {
    pub fn new() -> Self {
        Self {
            expense_id: None,
            description: String::new(),
            paid_by: String::new(),
            amount: 0.0,
            amount_formatted: String::from("0.00"),
            currency: String::from("USD"),
            date: String::new(),
            date_only: String::new(),
            split_ratios: Vec::new(),
            participants: Vec::new(),
            participant_splits: Vec::new(),
            participant_options: Vec::new(),
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
        self.amount_formatted = format!("{:.2}", amount);
        self
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn date(mut self, date: impl Into<String>) -> Self {
        let date_str = date.into();
        self.date_only = date_str.split('T').next().unwrap_or(&date_str).to_string();
        self.date = date_str;
        self
    }

    pub fn split_ratios(mut self, splits: Vec<Split>) -> Self {
        self.split_ratios = splits;
        self
    }

    pub fn participants(mut self, participants: Vec<(String, String)>) -> Self {
        // Precompute participant split data
        self.participant_splits = participants
            .iter()
            .map(|(id, name)| {
                let split = self
                    .split_ratios
                    .iter()
                    .find(|s| s.entity_id.to_string() == *id);
                let ratio_value = split
                    .map(|s| format!("{}/{}", s.ratio.numerator(), s.ratio.denominator()))
                    .unwrap_or_else(|| "0/1".to_string());
                let is_included = split.is_some();

                ParticipantSplit {
                    id: id.clone(),
                    name: name.clone(),
                    is_included,
                    ratio_value,
                }
            })
            .collect();

        // Precompute participant options with selected state
        self.participant_options = participants
            .iter()
            .map(|(id, name)| ParticipantOption {
                id: id.clone(),
                name: name.clone(),
                selected: id == &self.paid_by,
            })
            .collect();

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
