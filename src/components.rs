use crate::structs::Split;
/// UI Components for Borrow Checker
///
/// All components use the builder pattern for flexible construction
/// and return HTML strings styled with Tailwind CSS classes.
use maud::html;

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
        html! {
            nav class="bg-zinc-900 px-12 py-6 flex justify-between items-center border-b border-zinc-700" {
                div class="brand" {
                    h1 class="text-orange-500 font-bold tracking-[0.5rem] uppercase text-base" {
                        "Borrow Checker"
                    }
                }
                @if let Some(user_name) = self.current_user_name {
                    div class="flex items-center gap-6" {
                        @if !self.group_members.is_empty() {
                            div class="flex items-center gap-3" {
                                span class="text-zinc-500 text-sm" { "Group:" }
                                div class="flex gap-2" {
                                    @for member in &self.group_members {
                                        span class="text-zinc-300 text-sm px-3 py-1 bg-zinc-800 rounded-full border border-zinc-700" {
                                            (member)
                                        }
                                    }
                                }
                            }
                        }
                        div class="flex items-center gap-2" {
                            span class="text-zinc-500 text-sm" { "Logged in as:" }
                            span class="text-orange-400 font-semibold text-sm" {
                                (user_name)
                            }
                        }
                    }
                }
            }
        }.into_string()
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Transaction {
    expense_id: Option<String>,
    description: String,
    payer_name: String,
    total_amount: f64,
    currency: String,
    date: String,
    status_label: String,
    status_color: String,
    user_amount: f64,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            expense_id: None,
            description: String::new(),
            payer_name: String::new(),
            total_amount: 0.0,
            currency: String::from("USD"),
            date: String::new(),
            status_label: String::new(),
            status_color: String::new(),
            user_amount: 0.0,
        }
    }

    pub fn expense_id(mut self, id: impl Into<String>) -> Self {
        self.expense_id = Some(id.into());
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
        self.status_label = String::from("YOU BORROWED");
        self.status_color = String::from("text-red-500");
        self.user_amount = -amount;
        self
    }

    pub fn lent(mut self, amount: f64) -> Self {
        self.status_label = String::from("YOU LENT");
        self.status_color = String::from("text-green-400");
        self.user_amount = amount;
        self
    }

    pub fn build(self) -> String {
        let expense_id = self.expense_id.unwrap_or_else(|| "unknown".to_string());
        let amount_display = if self.user_amount < 0.0 {
            format!("-{} {:.2}", self.currency, self.user_amount.abs())
        } else {
            format!("{} {:.2}", self.currency, self.user_amount)
        };

        html! {
            button
                class="expense-item group relative grid grid-cols-[1fr_auto_auto] items-center px-12 py-6 border-b border-zinc-700 cursor-pointer transition-colors hover:bg-zinc-900 w-full text-left"
                type="button"
                name="expenseId"
                value=(expense_id)
                hx-tauri-invoke="get_expense"
                hx-target="#expense-list" {

                // Hover background effect
                div class="absolute inset-0 bg-gradient-to-r from-transparent to-white/[0.03] scale-x-0 origin-left transition-transform duration-600 ease-out group-hover:scale-x-100 pointer-events-none" {}

                // Main info
                div class="relative z-10" {
                    h3 class="text-xl font-light mb-1" {
                        (self.description)
                    }
                    span class="font-mono text-xs text-gray-400 uppercase" {
                        "Paid by: " (self.payer_name) " • Total: " (self.currency) " " (format!("{:.2}", self.total_amount)) " • " (self.date)
                    }
                }

                // Status
                div class="relative z-10 text-right mr-8 font-mono" {
                    span class="block text-[0.65rem] text-gray-500 uppercase" {
                        (self.status_label)
                    }
                    span class=(format!("text-lg {}", self.status_color)) {
                        (amount_display)
                    }
                }

                // Chevron
                div class="relative z-10" {
                    svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" {
                        path d="M9 18l6-6-6-6" {}
                    }
                }
            }
        }.into_string()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LedgerHeader {
    ledger_name: String,
    balances: Vec<(String, f64)>, // (user_name, amount) pairs
    currency: String,
    ledgers: Vec<(String, String)>, // (id, name) pairs
}

impl LedgerHeader {
    pub fn new() -> Self {
        Self {
            ledger_name: String::new(),
            balances: Vec::new(),
            currency: String::from("USD"),
            ledgers: Vec::new(),
        }
    }

    pub fn ledger_name(mut self, name: impl Into<String>) -> Self {
        self.ledger_name = name.into();
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
        html! {
            header class="px-12 py-8 flex justify-between items-end border-b border-zinc-700 bg-gradient-to-b from-zinc-900 to-transparent" id="ledger-header" {
                div class="title-group" {
                    span class="font-mono text-xs text-orange-500 uppercase tracking-wide" {
                        "LEDGER"
                    }
                    div class="flex items-center gap-2" {
                        select
                            class="text-5xl font-light uppercase tracking-tight leading-tight bg-transparent text-white border-none outline-none cursor-pointer flex-shrink-0"
                            style="-webkit-appearance: none; -moz-appearance: none; appearance: none; width: fit-content;"
                            name="ledger_id"
                            hx-tauri-invoke="switch_ledger"
                            hx-target="#main-content" {
                            @for (id, name) in &self.ledgers {
                                option value=(id) selected[name == &self.ledger_name] {
                                    (name)
                                }
                            }
                        }
                        span class="pointer-events-none opacity-40 flex-shrink-0" {
                            svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" {
                                path d="M7 10l5 5 5-5" {}
                            }
                        }
                    }
                }

                div class="balance text-right" {
                    span class="font-mono text-xs text-gray-500 uppercase block mb-3" {
                        "YOUR BALANCES"
                    }
                    @if self.balances.is_empty() {
                        span class="font-mono text-xl text-gray-600" {
                            "All settled up"
                        }
                    } @else {
                        div class="space-y-2" {
                            @for (user_name, amount) in &self.balances {
                                div class="flex items-center justify-end gap-3" {
                                    span class="text-sm text-gray-400" {
                                        (user_name)
                                    }
                                    @if *amount < 0.0 {
                                        span class="font-mono text-lg text-red-500" {
                                            (format!("-{} {:.2}", self.currency, amount.abs()))
                                        }
                                    } @else if *amount > 0.0 {
                                        span class="font-mono text-lg text-green-400" {
                                            (format!("{} {:.2}", self.currency, amount))
                                        }
                                    } @else {
                                        span class="font-mono text-lg text-gray-600" {
                                            (format!("{} 0.00", self.currency))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }.into_string()
    }
}

impl Default for LedgerHeader {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExpenseForm {
    expense_id: Option<String>,
    description: String,
    paid_by: String,
    amount: f64,
    currency: String,
    date: String,
    split_ratios: Vec<Split>,
    participants: Vec<(String, String)>, // (id, display_name) pairs
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
        }
    }

    pub fn expense_id(mut self, id: impl Into<String>) -> Self {
        self.expense_id = Some(id.into());
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
        let is_edit = self.expense_id.is_some();
        let form_title = if is_edit {
            "Edit Expense"
        } else {
            "Add Expense"
        };
        let submit_label = if is_edit {
            "Update Expense"
        } else {
            "Create Expense"
        };

        // Extract date only (without time) for the date input
        let date_only = self.date.split('T').next().unwrap_or(&self.date);

        html! {
            div class="flex" style="height: calc(100vh - 280px);" {
                // Rotated title sidebar - sticky positioning
                div class="sticky flex flex-col items-center justify-end bg-gradient-to-b from-zinc-900 to-zinc-950 border-r border-zinc-700" {
                    h2 class="text-2xl font-bold tracking-[0.2em] uppercase whitespace-nowrap origin-center text-zinc-500"
                        style="font-family: 'Space Grotesk', sans-serif; writing-mode: vertical-rl; transform: rotate(180deg); padding: 16px 12px;" {
                        (form_title)
                    }
                }

                // Form content - scrollable
                div class="flex-1 px-8 py-6 overflow-y-auto" {
                    // Back button
                    div class="mb-6" {
                        button
                            class="text-zinc-400 hover:text-orange-500 flex items-center gap-2 transition-colors"
                            hx-tauri-invoke="render_transactions"
                            hx-target="#expense-list" {
                            svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" {
                                path d="M15 18l-6-6 6-6" {}
                            }
                            span { "Back to Transactions" }
                        }
                    }

                    // Form
                    form class="space-y-6" {
                    // Description field
                    div class="form-group" {
                        label class="block text-sm font-mono text-zinc-400 uppercase mb-2" for="description" {
                            "Description"
                        }
                        input
                            type="text"
                            name="description"
                            id="description"
                            value=(self.description)
                            required
                            class="w-full bg-zinc-800 border border-zinc-700 rounded px-4 py-3 text-white focus:border-orange-500 focus:outline-none transition-colors";
                    }

                    // Amount and Currency
                    div class="grid grid-cols-2 gap-4" {
                        div class="form-group" {
                            label class="block text-sm font-mono text-zinc-400 uppercase mb-2" for="amount" {
                                "Amount"
                            }
                            input
                                type="number"
                                name="amount"
                                id="amount"
                                value=(format!("{:.2}", self.amount))
                                step="0.01"
                                min="0"
                                required
                                class="w-full bg-zinc-800 border border-zinc-700 rounded px-4 py-3 text-white focus:border-orange-500 focus:outline-none transition-colors";
                        }

                        div class="form-group" {
                            label class="block text-sm font-mono text-zinc-400 uppercase mb-2" for="currency" {
                                "Currency"
                            }
                            select
                                name="currency"
                                id="currency"
                                required
                                class="w-full bg-zinc-800 border border-zinc-700 rounded px-4 py-3 text-white focus:border-orange-500 focus:outline-none transition-colors" {
                                option value="USD" selected[self.currency == "USD"] { "USD" }
                                option value="EUR" selected[self.currency == "EUR"] { "EUR" }
                                option value="GBP" selected[self.currency == "GBP"] { "GBP" }
                                option value="CHF" selected[self.currency == "CHF"] { "CHF" }
                                option value="JPY" selected[self.currency == "JPY"] { "JPY" }
                            }
                        }
                    }

                    // Paid by and Date
                    div class="grid grid-cols-2 gap-4" {
                        div class="form-group" {
                            label class="block text-sm font-mono text-zinc-400 uppercase mb-2" for="paid_by" {
                                "Paid By"
                            }
                            select
                                name="paid_by"
                                id="paid_by"
                                required
                                class="w-full bg-zinc-800 border border-zinc-700 rounded px-4 py-3 text-white focus:border-orange-500 focus:outline-none transition-colors" {
                                @for (id, name) in &self.participants {
                                    option value=(id) selected[id == &self.paid_by] {
                                        (name)
                                    }
                                }
                            }
                        }

                        div class="form-group" {
                            label class="block text-sm font-mono text-zinc-400 uppercase mb-2" for="date" {
                                "Date"
                            }
                            input
                                type="date"
                                name="date"
                                id="date"
                                value=(date_only)
                                required
                                class="w-full bg-zinc-800 border border-zinc-700 rounded px-4 py-3 text-white focus:border-orange-500 focus:outline-none transition-colors";
                        }
                    }

                    // Split ratios section
                    div class="form-group" {
                        label class="block text-sm font-mono text-zinc-400 uppercase mb-3" {
                            "Split Between"
                        }
                        div class="space-y-2" {
                            @for (participant_id, participant_name) in &self.participants {
                                @let split = self.split_ratios.iter().find(|s| s.entity_id.to_string() == *participant_id);
                                @let ratio_value = split.map(|s| format!("{}/{}", s.ratio.numerator(), s.ratio.denominator())).unwrap_or_else(|| "0/1".to_string());
                                @let is_included = split.is_some();

                                div class="flex items-center gap-4 bg-zinc-800 border border-zinc-700 rounded px-4 py-3" {
                                    input
                                        type="checkbox"
                                        name=(format!("split_include_{}", participant_id))
                                        id=(format!("split_include_{}", participant_id))
                                        checked[is_included]
                                        class="w-4 h-4 accent-orange-500";

                                    label for=(format!("split_include_{}", participant_id)) class="flex-1 text-white" {
                                        (participant_name)
                                    }

                                    input
                                        type="text"
                                        name=(format!("split_ratio_{}", participant_id))
                                        placeholder="1/1"
                                        value=(ratio_value)
                                        class="w-24 bg-zinc-900 border border-zinc-600 rounded px-3 py-1 text-white text-sm focus:border-orange-500 focus:outline-none";
                                }
                            }
                        }
                    }

                    // Action buttons
                    div class="flex gap-4 pt-4" {
                        button
                            type="button"
                            class="flex-1 bg-zinc-800 hover:bg-zinc-700 text-white font-semibold py-3 px-6 rounded transition-colors border border-zinc-700"
                            hx-tauri-invoke="render_transactions"
                            hx-target="#expense-list" {
                            "Cancel"
                        }

                        button
                            type="submit"
                            class="flex-1 bg-orange-500 hover:bg-orange-600 text-white font-semibold py-3 px-6 rounded transition-colors"
                            hx-tauri-invoke=(if is_edit { "update_expense" } else { "create_expense" })
                            hx-target="#main-content" {
                            (submit_label)
                        }
                    }
                }
            }
        }
        }.into_string()
    }
}

impl Default for ExpenseForm {
    fn default() -> Self {
        Self::new()
    }
}
