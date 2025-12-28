use crate::structs::{Split, Transaction};
use std::collections::HashMap;
use log::warn;
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

            let share = split.user_share(transaction);

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
            return split.user_share(transaction);
        }
    }
    0.0
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

#[cfg(test)]
mod tests {
    use crate::structs::SplitType;

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
                    ratio: Some(Rational::new(numer, denom)),
                    amount: None
                })
                .collect(),
            split_type: SplitType::Ratio,
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
}
