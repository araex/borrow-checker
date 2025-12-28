use crate::structs::{Group, Ledger, Transaction};
use std::error::Error;
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    /// Repository access error (opening, finding refs/commits, reading objects)
    RepositoryError(String),

    /// File I/O or encoding error (reading/writing files, UTF-8 decoding)
    DataError(String),

    /// Failed to parse ledger from storage
    ParseLedger {
        ledger_name: String,
        message: String,
    },

    /// Requested object not found (e.g. ledger id not found)
    NotFound(String),

    /// Operation is not supported by this persistence implementation
    UnsupportedOperation(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::RepositoryError(s) => write!(f, "Repository error: {}", s),
            PersistenceError::DataError(s) => write!(f, "Data error: {}", s),
            PersistenceError::ParseLedger {
                ledger_name,
                message,
            } => {
                write!(f, "Failed to parse ledger '{}': {}", ledger_name, message)
            }
            PersistenceError::NotFound(s) => write!(f, "Not found: {}", s),
            PersistenceError::UnsupportedOperation(s) => write!(f, "Unsupported operation: {}", s),
        }
    }
}

impl Error for PersistenceError {}

impl From<std::io::Error> for PersistenceError {
    fn from(e: std::io::Error) -> Self {
        PersistenceError::DataError(format!("{}", e))
    }
}

impl From<std::str::Utf8Error> for PersistenceError {
    fn from(e: std::str::Utf8Error) -> Self {
        PersistenceError::DataError(format!("UTF-8 decode error: {}", e))
    }
}

impl From<toml::de::Error> for PersistenceError {
    fn from(e: toml::de::Error) -> Self {
        PersistenceError::DataError(format!("TOML parse error: {}", e))
    }
}

impl From<git2::Error> for PersistenceError {
    fn from(e: git2::Error) -> Self {
        PersistenceError::RepositoryError(format!("{}", e))
    }
}

// ============================================================================
// Result Types and Supporting Structures
// ============================================================================

/// Result of a refresh operation
#[derive(Debug)]
pub struct RefreshResult {
    /// Whether anything has changed in the remote storage
    pub has_changes: bool,
}

// ============================================================================
// Persistence Trait
// ============================================================================

/// Trait for persistence operations
///
/// Handles all data storage and retrieval (CRUD operations).
/// Storage-agnostic - implementation details are hidden behind this interface.
/// Returns raw structs without validation or calculation.
pub trait PersistenceRepository {
    // ------------------------------------------------------------------------
    // Group Operations
    // ------------------------------------------------------------------------

    /// Load the group configuration containing all entities
    fn load_group(&self) -> Result<Group, PersistenceError>;

    /// Persist group configuration changes (including all entities)
    fn save_group(&self, group: &Group) -> Result<(), PersistenceError>;

    // ------------------------------------------------------------------------
    // Ledger Operations
    // ------------------------------------------------------------------------

    /// Scan repository and return all ledgers
    fn list_ledgers(&self) -> Result<Vec<Ledger>, PersistenceError>;

    /// Create a new ledger in the repository
    ///
    /// Returns the UUID of the newly created ledger
    fn create_ledger(&self, ledger: Ledger) -> Result<Uuid, PersistenceError>;

    /// Update ledger metadata (display_name, participants)
    fn update_ledger(&self, ledger: Ledger) -> Result<(), PersistenceError>;

    /// Remove a ledger and optionally its transactions
    fn delete_ledger(&self, id: Uuid) -> Result<(), PersistenceError>;

    // ------------------------------------------------------------------------
    // Transaction Operations
    // ------------------------------------------------------------------------

    /// Get all transactions for a specific ledger
    fn list_transactions(&self, ledger_id: Uuid) -> Result<Vec<Transaction>, PersistenceError>;

    /// Add a new transaction to a ledger
    ///
    /// Returns the UUID of the created transaction
    fn create_transaction(
        &self,
        ledger_id: Uuid,
        transaction: Transaction,
    ) -> Result<Uuid, PersistenceError>;

    /// Modify an existing transaction
    fn update_transaction(
        &self,
        ledger_id: Uuid,
        transaction: Transaction,
    ) -> Result<(), PersistenceError>;

    /// Remove a transaction from a ledger
    fn delete_transaction(
        &self,
        ledger_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<(), PersistenceError>;

    // ------------------------------------------------------------------------
    // Storage Operations
    // ------------------------------------------------------------------------

    /// Refreshes local data from remote storage
    fn refresh(&self) -> Result<RefreshResult, PersistenceError>;
}
