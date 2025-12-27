use crate::structs::{Entity, Group, Ledger, LedgerWithTransactions, Split, Transaction};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    /// Could not open repository at the given path (message contains path + detail)
    RepoOpen(String),

    /// Generic Git error (stringified message)
    Git(String),

    /// I/O error (stringified)
    Io(String),

    /// UTF-8 decoding error (stringified)
    Utf8(String),

    /// TOML deserialization error (stringified)
    Toml(String),

    /// Requested object not found (e.g. ledger id not found)
    NotFound(String),

    /// An object in the tree had an unexpected object type
    InvalidObjectType(String),

    /// Operation is not supported by this persistence implementation
    UnsupportedOperation(String),

    /// Ledger parsing failed for a specific ledger marker
    ParseLedger {
        ledger_name: String,
        message: String,
    },

    /// Generic other error with free-form message
    Other(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::RepoOpen(s) => write!(f, "Repo open error: {}", s),
            PersistenceError::Git(s) => write!(f, "Git error: {}", s),
            PersistenceError::Io(s) => write!(f, "IO error: {}", s),
            PersistenceError::Utf8(s) => write!(f, "UTF-8 decode error: {}", s),
            PersistenceError::Toml(s) => write!(f, "TOML parse error: {}", s),
            PersistenceError::NotFound(s) => write!(f, "Not found: {}", s),
            PersistenceError::InvalidObjectType(s) => write!(f, "Invalid object type: {}", s),
            PersistenceError::UnsupportedOperation(s) => write!(f, "Unsupported operation: {}", s),
            PersistenceError::ParseLedger {
                ledger_name,
                message,
            } => {
                write!(f, "Failed to parse ledger '{}': {}", ledger_name, message)
            }
            PersistenceError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl Error for PersistenceError {}

// Conversion helpers (implement in traits to make them easily usable by implementations)
impl From<std::io::Error> for PersistenceError {
    fn from(e: std::io::Error) -> Self {
        PersistenceError::Io(format!("{}", e))
    }
}

impl From<std::str::Utf8Error> for PersistenceError {
    fn from(e: std::str::Utf8Error) -> Self {
        PersistenceError::Utf8(format!("{}", e))
    }
}

impl From<toml::de::Error> for PersistenceError {
    fn from(e: toml::de::Error) -> Self {
        PersistenceError::Toml(format!("{}", e))
    }
}

impl From<git2::Error> for PersistenceError {
    fn from(e: git2::Error) -> Self {
        PersistenceError::Git(format!("{}", e))
    }
}

#[derive(Debug)]
pub enum BusinessLogicError {}

// ============================================================================
// Result Types and Supporting Structures
// ============================================================================

/// Result of a refresh operation
#[derive(Debug)]
pub struct RefreshResult {
    /// Whether anything has changed in the remote storage
    pub has_changes: bool,
}

/// Represents a payment to settle debts
#[derive(Debug)]
pub struct Settlement {
    /// Who pays
    pub from_entity: Uuid,
    /// Who receives
    pub to_entity: Uuid,
    /// How much
    pub amount: f64,
    /// Currency code
    pub currency: String,
}

/// Result of validation operations
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// List of validation errors (empty if is_valid is true)
    pub errors: Vec<ValidationError>,
}

/// A single validation error
#[derive(Debug)]
pub struct ValidationError {
    /// Field name or path (e.g., "split_ratios[0].entity_id")
    pub field: String,
    /// Human-readable error message
    pub message: String,
    /// Type of validation error
    pub error_type: ValidationErrorType,
}

/// Types of validation errors
#[derive(Debug)]
pub enum ValidationErrorType {
    /// Required field is missing
    MissingField,
    /// Invalid format
    InvalidFormat,
    /// UUID reference doesn't exist
    InvalidReference,
    /// Value is out of range or invalid
    InvalidValue,
    /// Duplicate ID found
    DuplicateValue,
    /// Sum mismatch (e.g., ratios don't sum to 1)
    SumMismatch,
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

// ============================================================================
// Validation Trait
// ============================================================================

/// Trait for validation operations
///
/// Validates data structures against business rules.
/// Stateless - pure functions that check data integrity.
/// Does not persist data or perform calculations.
pub trait Validator {
    // ------------------------------------------------------------------------
    // Entity-level Validation
    // ------------------------------------------------------------------------

    /// Validate group configuration
    ///
    /// Checks:
    /// - At least one entity exists
    /// - All entity IDs are unique
    /// - All entity display names are not empty
    fn validate_group(&self, group: &Group) -> ValidationResult;

    /// Validate ledger metadata
    ///
    /// Checks:
    /// - Ledger ID is present
    /// - Display name is not empty
    /// - All participants exist in group
    /// - Participants list is not empty
    fn validate_ledger(&self, ledger: &Ledger, group: &Group) -> ValidationResult;

    /// Validate all aspects of a transaction
    ///
    /// Checks:
    /// - Transaction ID is present
    /// - Paid-by entity exists in group and is ledger participant
    /// - All split entities exist in group and are ledger participants
    /// - Split ratios are positive and sum to ~1 (within tolerance)
    /// - Currency code is valid ISO 4217
    /// - Amount is positive
    /// - Description is not empty
    /// - Datetime is valid
    fn validate_transaction(
        &self,
        transaction: &Transaction,
        ledger: &Ledger,
        group: &Group,
    ) -> ValidationResult;

    // ------------------------------------------------------------------------
    // Field-level Validation
    // ------------------------------------------------------------------------

    /// Ensure a UUID reference exists in group entities
    fn validate_entity_reference(
        &self,
        entity_id: Uuid,
        group: &Group,
    ) -> Result<(), ValidationError>;

    /// Validate ISO 4217 currency codes (3-letter codes)
    fn validate_currency(&self, code: &str) -> Result<(), ValidationError>;

    /// Ensure sum of all ratios equals 1 (within tolerance of 0.001)
    fn validate_split_ratios_sum(&self, ratios: &[Split]) -> Result<(), ValidationError>;
}

// ============================================================================
// Business Logic Trait
// ============================================================================

/// Trait for business logic operations
///
/// Performs calculations and data transformations.
/// Balance calculations, settlement optimization, share computations.
/// Does not validate or persist (those are separate concerns).
pub trait BusinessLogic {
    /// Calculate who owes whom from a user's perspective
    ///
    /// Returns a map of entity UUID to amount where:
    /// - Positive values = they owe you
    /// - Negative values = you owe them
    ///
    /// Algorithm: For each transaction:
    /// - If user paid: they are owed by each other participant for their share
    /// - If user didn't pay: they owe the payer their share
    fn calculate_balances(
        &self,
        ledger_id: Uuid,
        user_id: Uuid,
    ) -> Result<HashMap<Uuid, f64>, BusinessLogicError>;

    /// Calculate a user's share of a transaction
    ///
    /// Algorithm: Find user's ratio in split_ratios, multiply by transaction amount
    /// Returns user's share amount (0.0 if user not in split)
    fn get_user_share(&self, transaction: &Transaction, user_id: Uuid) -> f64;

    /// Normalize split ratios to sum to 1
    ///
    /// Algorithm: Calculate total ratio, divide each by total
    fn normalize_split_ratios(&self, ratios: Vec<Split>) -> Vec<Split>;

    /// Optimize debt settlement (who pays whom, minimizing transactions)
    ///
    /// Returns list of settlements (from_entity, to_entity, amount)
    fn calculate_settlements(&self, balances: HashMap<Uuid, f64>) -> Vec<Settlement>;
}
