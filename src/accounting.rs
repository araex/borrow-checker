use crate::structs::{Split, Transaction};
use std::collections::HashMap;
use uuid::Uuid;

/// Calculate who owes whom from a user's perspective
///
/// Returns a map of entity UUID to amount where:
/// - Positive values = they owe you
/// - Negative values = you owe them
///
/// Algorithm: For each transaction:
/// - If user paid: they are owed by each other participant for their share
/// - If user didn't pay: they owe the payer their share
pub fn calculate_balances(
    transactions: &[Transaction],
    user_id: Uuid,
) -> HashMap<Uuid, f64> {
    let mut balances: HashMap<Uuid, f64> = HashMap::new();

    for transaction in transactions {
        let amount = transaction.amount;
        let paid_by = transaction.paid_by_entity;

        // Calculate shares for each participant
        for split in &transaction.split_ratios {
            let entity_id = split.entity_id;
            let ratio = split.ratio.decimal_value();
            let share = amount * ratio;

            if entity_id == user_id {
                // User's own share
                if paid_by == user_id {
                    // User paid for themselves - no net change for themselves
                    continue;
                } else {
                    // User owes the payer their share (negative balance)
                    *balances.entry(paid_by).or_insert(0.0) -= share;
                }
            } else if paid_by == user_id {
                // User paid, so others owe them (positive balance)
                *balances.entry(entity_id).or_insert(0.0) += share;
            }
        }
    }

    balances
}

/// Calculate a user's share of a transaction
///
/// Algorithm: Find user's ratio in split_ratios, multiply by transaction amount
/// Returns user's share amount (0.0 if user not in split)
pub fn get_user_share(transaction: &Transaction, user_id: Uuid) -> f64 {
    for split in &transaction.split_ratios {
        if split.entity_id == user_id {
            let ratio = split.ratio.decimal_value();
            return transaction.amount * ratio;
        }
    }
    0.0
}

/// Calculate the net amount for a user in a transaction
/// Returns:
/// - Positive if user lent money (paid for others)
/// - Negative if user borrowed money (others paid for them)
/// - Zero if user paid exactly their share or wasn't involved
pub fn calculate_user_amount(transaction: &Transaction, user_id: Uuid) -> f64 {
    let user_share = get_user_share(transaction, user_id);
    
    if user_share == 0.0 {
        return 0.0; // User not involved in this transaction
    }
    
    if transaction.paid_by_entity == user_id {
        // User paid, so they lent: (total - their share)
        transaction.amount - user_share
    } else {
        // Someone else paid, so user owes their share (negative)
        -user_share
    }
}

/// Get the primary currency from a list of transactions
///
/// Returns the currency of the first transaction, or a default "USD" if no transactions exist
pub fn get_primary_currency(transactions: &[Transaction]) -> String {
    transactions
        .first()
        .map(|txn| txn.currency_iso_4217.clone())
        .unwrap_or_else(|| String::from("USD"))
}

/// Normalize split ratios to sum to 1
///
/// Algorithm: Calculate total ratio, divide each by total
pub fn normalize_split_ratios(ratios: Vec<Split>) -> Vec<Split> {
    // Calculate total ratio
    let total: f64 = ratios.iter().map(|split| split.ratio.decimal_value()).sum();

    if total == 0.0 {
        return ratios;
    }

    // Normalize each ratio
    ratios
        .into_iter()
        .map(|split| {
            let current_ratio = split.ratio.decimal_value();
            let normalized = current_ratio / total;
            Split {
                entity_id: split.entity_id,
                ratio: rational::Rational::new((normalized * 1_000_000.0) as i64, 1_000_000),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rational::Rational;

    fn create_test_transaction(
        paid_by: Uuid,
        amount: f64,
        splits: Vec<(Uuid, i64, i64)>,
    ) -> Transaction {
        use toml::value::Datetime;

        Transaction {
            id: Uuid::new_v4(),
            description: "Test".to_string(),
            paid_by_entity: paid_by,
            currency_iso_4217: "EUR".to_string(),
            amount,
            transaction_datetime_rfc_3339: Datetime {
                date: Some(toml::value::Date {
                    year: 2025,
                    month: 1,
                    day: 1,
                }),
                time: None,
                offset: None,
            },
            split_ratios: splits
                .into_iter()
                .map(|(id, numer, denom)| Split {
                    entity_id: id,
                    ratio: Rational::new(numer, denom),
                })
                .collect(),
        }
    }

    #[test]
    fn test_get_user_share() {
        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        let transaction =
            create_test_transaction(other_id, 100.0, vec![(user_id, 1, 2), (other_id, 1, 2)]);

        let share = get_user_share(&transaction, user_id);

        assert!((share - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_get_user_share_not_participant() {
        let user_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let non_participant = Uuid::new_v4();

        let transaction =
            create_test_transaction(other_id, 100.0, vec![(user_id, 1, 2), (other_id, 1, 2)]);

        let share = get_user_share(&transaction, non_participant);

        assert_eq!(share, 0.0);
    }

    #[test]
    fn test_calculate_balances_simple() {
        let user_id = Uuid::new_v4();
        let friend_id = Uuid::new_v4();

        // User paid 100, split evenly
        let transaction =
            create_test_transaction(user_id, 100.0, vec![(user_id, 1, 2), (friend_id, 1, 2)]);

        let balances = calculate_balances(&[transaction], user_id);

        // Friend owes user 50
        assert_eq!(balances.len(), 1);
        assert!((balances[&friend_id] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_split_ratios() {
        let entity1 = Uuid::new_v4();
        let entity2 = Uuid::new_v4();

        let splits = vec![
            Split {
                entity_id: entity1,
                ratio: Rational::new(2, 1),
            },
            Split {
                entity_id: entity2,
                ratio: Rational::new(1, 1),
            },
        ];

        let normalized = normalize_split_ratios(splits);

        let total: f64 = normalized.iter().map(|s| s.ratio.decimal_value()).sum();

        assert!((total - 1.0).abs() < 0.01);
    }
}
