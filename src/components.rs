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
