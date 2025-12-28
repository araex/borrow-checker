//! Validation module for borrow-checker
//!
//! This module provides validation functions for the borrow-checker application.
//! Previously implemented as a trait, the validation logic is now provided as
//! free functions since there's no need for polymorphism or different implementations.
//!
//! The validation functions are stateless and pure, checking data integrity
//! against business rules without persisting data or performing calculations.
//!
//! # Main validation functions
//!
//! - [`validate_group`] - Validates group configuration and entities
//! - [`validate_ledger`] - Validates ledger metadata and participants
//! - [`validate_transaction`] - Validates transaction data comprehensively
//!
//! # Helper validation functions
//!
//! - [`validate_entity_reference`] - Checks if an entity ID exists in the group
//! - [`validate_currency`] - Validates ISO 4217 currency codes
//! - [`validate_split_ratios_sum`] - Ensures split ratios sum to 1

use crate::structs::{Group, Ledger, Split, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

// ============================================================================
// Validation Types
// ============================================================================

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
// Group Validation
// ============================================================================

/// Validate group configuration
///
/// Checks:
/// - At least one entity exists
/// - All entity IDs are unique
/// - All entity display names are not empty
pub fn validate_group(group: &Group) -> ValidationResult {
    let mut errors = Vec::new();

    // Check that at least one entity exists
    if group.entities.is_empty() {
        errors.push(ValidationError {
            field: "entities".to_string(),
            message: "Group must have at least one entity".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    // Check for duplicate entity IDs
    let mut seen_ids = HashSet::new();
    for (idx, entity) in group.entities.iter().enumerate() {
        if !seen_ids.insert(entity.id) {
            errors.push(ValidationError {
                field: format!("entities[{}].id", idx),
                message: format!("Duplicate entity ID: {}", entity.id),
                error_type: ValidationErrorType::DuplicateValue,
            });
        }

        // Check that display name is not empty
        if entity.display_name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("entities[{}].display_name", idx),
                message: "Entity display name cannot be empty".to_string(),
                error_type: ValidationErrorType::InvalidValue,
            });
        }
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
    }
}

// ============================================================================
// Ledger Validation
// ============================================================================

/// Validate ledger metadata
///
/// Checks:
/// - Ledger ID is present (non-nil UUID)
/// - Display name is not empty
/// - All participants exist in group
/// - Participants list is not empty
pub fn validate_ledger(ledger: &Ledger, group: &Group) -> ValidationResult {
    let mut errors = Vec::new();

    // Check that ID is not nil
    if ledger.id == Uuid::nil() {
        errors.push(ValidationError {
            field: "id".to_string(),
            message: "Ledger ID cannot be nil".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    // Check that display name is not empty
    if ledger.display_name.trim().is_empty() {
        errors.push(ValidationError {
            field: "display_name".to_string(),
            message: "Ledger display name cannot be empty".to_string(),
            error_type: ValidationErrorType::InvalidValue,
        });
    }

    // Check that participants list is not empty
    if ledger.participants.is_empty() {
        errors.push(ValidationError {
            field: "participants".to_string(),
            message: "Ledger must have at least one participant".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    // Check that all participants exist in group
    for (idx, participant_id) in ledger.participants.iter().enumerate() {
        if let Err(err) = validate_entity_reference(*participant_id, group) {
            errors.push(ValidationError {
                field: format!("participants[{}]", idx),
                message: err.message,
                error_type: err.error_type,
            });
        }
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
    }
}

// ============================================================================
// Transaction Validation
// ============================================================================

/// Validate all aspects of a transaction
///
/// Checks:
/// - Transaction ID is present (non-nil UUID)
/// - Paid-by entity exists in group and is ledger participant
/// - All split entities exist in group and are ledger participants
/// - Split ratios are positive and sum to ~1 (within tolerance)
/// - Currency code is valid ISO 4217
/// - Amount is positive
/// - Description is not empty
/// - Datetime is valid (handled by type system)
pub fn validate_transaction(
    transaction: &Transaction,
    ledger: &Ledger,
    group: &Group,
) -> ValidationResult {
    let mut errors = Vec::new();

    // Check that ID is not nil
    if transaction.id == Uuid::nil() {
        errors.push(ValidationError {
            field: "id".to_string(),
            message: "Transaction ID cannot be nil".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    // Check that description is not empty
    if transaction.description.trim().is_empty() {
        errors.push(ValidationError {
            field: "description".to_string(),
            message: "Transaction description cannot be empty".to_string(),
            error_type: ValidationErrorType::InvalidValue,
        });
    }

    // Check that amount is positive
    if transaction.amount <= 0.0 {
        errors.push(ValidationError {
            field: "amount".to_string(),
            message: format!("Transaction amount must be positive, got: {}", transaction.amount),
            error_type: ValidationErrorType::InvalidValue,
        });
    }

    // Validate currency code
    if let Err(err) = validate_currency(&transaction.currency_iso_4217) {
        errors.push(ValidationError {
            field: "currency_iso_4217".to_string(),
            message: err.message,
            error_type: err.error_type,
        });
    }

    // Validate paid_by entity
    if let Err(err) = validate_entity_reference(transaction.paid_by_entity, group) {
        errors.push(ValidationError {
            field: "paid_by_entity".to_string(),
            message: err.message,
            error_type: err.error_type,
        });
    } else {
        // Also check that paid_by entity is a ledger participant
        if !ledger.participants.contains(&transaction.paid_by_entity) {
            errors.push(ValidationError {
                field: "paid_by_entity".to_string(),
                message: format!(
                    "Entity {} is not a participant in this ledger",
                    transaction.paid_by_entity
                ),
                error_type: ValidationErrorType::InvalidReference,
            });
        }
    }

    // Validate split ratios
    if transaction.split_ratios.is_empty() {
        errors.push(ValidationError {
            field: "split_ratios".to_string(),
            message: "Transaction must have at least one split".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    for (idx, split) in transaction.split_ratios.iter().enumerate() {
        // Check that entity exists in group
        if let Err(err) = validate_entity_reference(split.entity_id, group) {
            errors.push(ValidationError {
                field: format!("split_ratios[{}].entity_id", idx),
                message: err.message,
                error_type: err.error_type,
            });
        } else {
            // Also check that split entity is a ledger participant
            if !ledger.participants.contains(&split.entity_id) {
                errors.push(ValidationError {
                    field: format!("split_ratios[{}].entity_id", idx),
                    message: format!(
                        "Entity {} is not a participant in this ledger",
                        split.entity_id
                    ),
                    error_type: ValidationErrorType::InvalidReference,
                });
            }
        }

        // Check that ratio is positive
        if split.ratio.is_some() && split.ratio.expect("How?!") <= rational::Rational::zero() {
            errors.push(ValidationError {
                field: format!("split_ratios[{}].ratio", idx),
                message: "Split ratio must be positive".to_string(),
                error_type: ValidationErrorType::InvalidValue,
            });
        }
    }

    // Validate that split ratios sum to 1
    if let Err(err) = validate_split_ratios_sum(&transaction.split_ratios) {
        errors.push(ValidationError {
            field: "split_ratios".to_string(),
            message: err.message,
            error_type: err.error_type,
        });
    }

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
    }
}

// ============================================================================
// Field-level Validation Helpers
// ============================================================================

/// Ensure a UUID reference exists in group entities
pub fn validate_entity_reference(entity_id: Uuid, group: &Group) -> Result<(), ValidationError> {
    if group.entities.iter().any(|e| e.id == entity_id) {
        Ok(())
    } else {
        Err(ValidationError {
            field: "entity_id".to_string(),
            message: format!("Entity with ID {} does not exist in group", entity_id),
            error_type: ValidationErrorType::InvalidReference,
        })
    }
}

/// Validate ISO 4217 currency codes (3-letter codes)
///
/// This is a simplified validation - checks format only.
/// For production, consider using a comprehensive currency code list.
pub fn validate_currency(code: &str) -> Result<(), ValidationError> {
    // Check that it's exactly 3 uppercase letters
    if code.len() != 3 {
        return Err(ValidationError {
            field: "currency".to_string(),
            message: format!(
                "Currency code must be 3 characters long (ISO 4217), got: '{}'",
                code
            ),
            error_type: ValidationErrorType::InvalidFormat,
        });
    }

    if !code.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError {
            field: "currency".to_string(),
            message: format!(
                "Currency code must contain only uppercase letters (ISO 4217), got: '{}'",
                code
            ),
            error_type: ValidationErrorType::InvalidFormat,
        });
    }

    // Optional: Add a list of valid ISO 4217 codes for stricter validation
    // For now, we accept any 3-letter uppercase code

    Ok(())
}

/// Ensure sum of all ratios equals 1 (within tolerance of 0.001)
pub fn validate_split_ratios_sum(ratios: &[Split]) -> Result<(), ValidationError> {
    if ratios.is_empty() {
        return Err(ValidationError {
            field: "ratios".to_string(),
            message: "Split ratios cannot be empty".to_string(),
            error_type: ValidationErrorType::MissingField,
        });
    }

    let sum: rational::Rational = ratios.iter().map(|s| s.ratio.expect("Expected ratio to be set")).sum();
    let one = rational::Rational::one();
    
    // Define tolerance as 1/1000
    let tolerance = rational::Rational::new(1, 1000);
    
    // Check if |sum - 1| <= tolerance
    let diff = if sum > one { sum - one } else { one - sum };
    
    if diff > tolerance {
        Err(ValidationError {
            field: "ratios".to_string(),
            message: format!(
                "Split ratios must sum to 1 (within tolerance of 0.001), got sum: {}",
                sum
            ),
            error_type: ValidationErrorType::SumMismatch,
        })
    } else {
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{Entity, SplitType};
    use test_context::{TestContext, test_context};
    use toml::value::Datetime;

    struct TestSplits {
        splits: Vec<Split>
    }

    impl TestContext for TestSplits {
        fn setup() -> Self {
                    let splits = vec![
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                ratio: Some(rational::Rational::new(1, 2)),
                amount: None,
            },
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                ratio: Some(rational::Rational::new(1, 2)),
                amount: None,
            },
        ];

        TestSplits { splits }
        }
    }

    fn create_test_group() -> Group {
        Group {
            entities: vec![
                Entity {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    display_name: "Alice".to_string(),
                },
                Entity {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                    display_name: "Bob".to_string(),
                },
            ],
        }
    }

    fn create_test_ledger() -> Ledger {
        Ledger {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000100").unwrap(),
            display_name: "Test Ledger".to_string(),
            participants: vec![
                Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            ],
        }
    }

    #[test]
    fn test_validate_group_success() {
        let group = create_test_group();
        let result = validate_group(&group);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_group_empty() {
        let group = Group {
            entities: vec![],
        };
        let result = validate_group(&group);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "entities");
    }

    #[test]
    fn test_validate_group_duplicate_id() {
        let duplicate_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let group = Group {
            entities: vec![
                Entity {
                    id: duplicate_id,
                    display_name: "Alice".to_string(),
                },
                Entity {
                    id: duplicate_id,
                    display_name: "Bob".to_string(),
                },
            ],
        };
        let result = validate_group(&group);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e.error_type, ValidationErrorType::DuplicateValue)));
    }

    #[test]
    fn test_validate_ledger_success() {
        let group = create_test_group();
        let ledger = create_test_ledger();
        let result = validate_ledger(&ledger, &group);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_ledger_invalid_participant() {
        let group = create_test_group();
        let mut ledger = create_test_ledger();
        ledger.participants.push(Uuid::parse_str("00000000-0000-0000-0000-000000000999").unwrap());
        
        let result = validate_ledger(&ledger, &group);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e.error_type, ValidationErrorType::InvalidReference)));
    }

    #[test]
    fn test_validate_currency_success() {
        assert!(validate_currency("USD").is_ok());
        assert!(validate_currency("EUR").is_ok());
        assert!(validate_currency("GBP").is_ok());
    }

    #[test]
    fn test_validate_currency_invalid() {
        assert!(validate_currency("US").is_err());
        assert!(validate_currency("USDD").is_err());
        assert!(validate_currency("usd").is_err());
        assert!(validate_currency("US1").is_err());
    }

    #[test]
    fn test_validate_split_ratios_sum_success() {
        let splits = vec![
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                ratio: Some(rational::Rational::new(1, 2)),
                amount: None
            },
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                ratio: Some(rational::Rational::new(1, 2)),
                amount: None
            },
        ];
        assert!(validate_split_ratios_sum(&splits).is_ok());
    }

    #[test]
    fn test_validate_split_ratios_sum_failure() {
        let splits = vec![
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                ratio: Some(rational::Rational::new(1, 3)),
                amount:None
            },
            Split {
                entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                ratio: Some(rational::Rational::new(1, 3)),
                amount:None
            },
        ];
        assert!(validate_split_ratios_sum(&splits).is_err());
    }

    #[test_context(TestSplits)]
    #[test]
    fn test_validate_transaction_success(sut: &mut TestSplits) {
        let group = create_test_group();
        let ledger = create_test_ledger();
        
        let transaction = Transaction {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000200").unwrap(),
            description: "Test transaction".to_string(),
            paid_by_entity: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            currency_iso_4217: "EUR".to_string(),
            amount: 100.0,
            transaction_datetime_rfc_3339: Datetime {
                date: Some(toml::value::Date { year: 2025, month: 12, day: 28 }),
                time: None,
                offset: None,
            },
            split_ratios: sut.splits.clone(),
            split_type: SplitType::Ratio
        };

        let result = validate_transaction(&transaction, &ledger, &group);
        assert!(result.is_valid, "Errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_transaction_invalid_amount() {
        let group = create_test_group();
        let ledger = create_test_ledger();
        
        let transaction = Transaction {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000200").unwrap(),
            description: "Test transaction".to_string(),
            paid_by_entity: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            currency_iso_4217: "EUR".to_string(),
            amount: -50.0,
            transaction_datetime_rfc_3339: Datetime {
                date: Some(toml::value::Date { year: 2025, month: 12, day: 28 }),
                time: None,
                offset: None,
            },
            split_ratios: vec![
                Split {
                    entity_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    ratio: Some(rational::Rational::new(1, 1)),
                    amount: None
                },
            ],
            split_type: SplitType::Ratio
        };

        let result = validate_transaction(&transaction, &ledger, &group);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "amount"));
    }
}
