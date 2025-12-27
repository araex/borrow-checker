/// UI Components for Borrow Checker
///
/// All components use the builder pattern for flexible construction
/// and return HTML strings styled with Tailwind CSS classes.
use maud::html;

pub struct Navigation {
    current_ledger: Option<String>,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            current_ledger: None,
        }
    }

    pub fn current_ledger(mut self, ledger_name: impl Into<String>) -> Self {
        self.current_ledger = Some(ledger_name.into());
        self
    }

    pub fn build(self) -> String {
        let ledger_display = self
            .current_ledger
            .unwrap_or_else(|| "Select Ledger".to_string());

        html! {
            nav class="bg-zinc-900 px-12 py-6 flex justify-between items-center border-b border-zinc-700" {
                div class="brand" {
                    h1 class="text-orange-500 font-bold tracking-[0.5rem] uppercase text-base" {
                        "Borrow Checker"
                    }
                }

                div class="breadcrumb flex items-center gap-4 font-mono text-sm" {
                    select
                        class="bg-zinc-800 text-gray-200 border border-zinc-700 px-4 py-2 cursor-pointer transition-colors hover:text-orange-500 hover:border-orange-500"
                        name="ledger_id"
                        hx-tauri-invoke="switch_ledger"
                        hx-target="#main-content" {
                        option selected { (ledger_display) }
                    }
                }
            }
        }.into_string()
    }
}

impl Default for Navigation {
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
            div 
                class="expense-item group relative grid grid-cols-[1fr_auto_auto] items-center px-12 py-6 border-b border-zinc-700 cursor-pointer transition-colors hover:bg-zinc-900"
                hx-tauri-invoke=(format!("get_expense:{}", expense_id))
                hx-target="#main-content" {
                
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
