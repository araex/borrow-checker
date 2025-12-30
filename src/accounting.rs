use crate::structs::Transaction;
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
pub fn calculate_balances(transactions: &[Transaction], user_id: Uuid) -> HashMap<Uuid, f64> {
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

/// Calculate optimal settlement payments to minimize number of transactions
///
/// This function takes all the debts between people and calculates the minimum
/// number of payments needed to settle all debts.
///
/// Algorithm:
/// 1. Calculate net balance for each person (positive = owed money, negative = owes money)
/// 2. Match creditors with debtors optimally
/// 3. Return list of payments that settle all debts
pub fn calculate_settlement_payments(
    transactions: &[Transaction],
    entity_names: &HashMap<Uuid, String>,
) -> Vec<(String, String, f64, String)> {
    // Calculate net balance for each entity across all transactions
    let mut net_balances: HashMap<Uuid, (f64, String)> = HashMap::new();
    
    for transaction in transactions {
        let amount = transaction.amount;
        let paid_by = transaction.paid_by_entity;
        let currency = &transaction.currency_iso_4217;
        
        for split in &transaction.split_ratios {
            let entity_id = split.entity_id;
            let ratio = split.ratio.decimal_value();
            let share = amount * ratio;
            
            if entity_id == paid_by {
                // Entity paid, so they are owed (amount - their share)
                let net = amount - share;
                let entry = net_balances.entry(entity_id).or_insert((0.0, currency.clone()));
                entry.0 += net;
            } else {
                // Entity owes their share to the payer
                let entry = net_balances.entry(entity_id).or_insert((0.0, currency.clone()));
                entry.0 -= share;
                
                let payer_entry = net_balances.entry(paid_by).or_insert((0.0, currency.clone()));
                payer_entry.0 += share;
            }
        }
    }
    
    // Separate into creditors (owed money) and debtors (owe money)
    let mut creditors: Vec<(Uuid, f64, String)> = Vec::new();
    let mut debtors: Vec<(Uuid, f64, String)> = Vec::new();
    
    for (entity_id, (balance, currency)) in net_balances {
        if balance > 0.01 {
            creditors.push((entity_id, balance, currency));
        } else if balance < -0.01 {
            debtors.push((entity_id, -balance, currency));
        }
    }
    
    // Calculate optimal payments
    let mut payments = Vec::new();
    let mut creditor_idx = 0;
    let mut debtor_idx = 0;
    
    while creditor_idx < creditors.len() && debtor_idx < debtors.len() {
        let (creditor_id, mut creditor_amount, creditor_currency) = creditors[creditor_idx].clone();
        let (debtor_id, mut debtor_amount, _debtor_currency) = debtors[debtor_idx].clone();
        
        let payment_amount = creditor_amount.min(debtor_amount);
        
        let from_name = entity_names.get(&debtor_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let to_name = entity_names.get(&creditor_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        
        payments.push((from_name, to_name, payment_amount, creditor_currency.clone()));
        
        creditor_amount -= payment_amount;
        debtor_amount -= payment_amount;
        
        if creditor_amount < 0.01 {
            creditor_idx += 1;
        } else {
            creditors[creditor_idx].1 = creditor_amount;
        }
        
        if debtor_amount < 0.01 {
            debtor_idx += 1;
        } else {
            debtors[debtor_idx].1 = debtor_amount;
        }
    }
    
    payments
}

/// Get all unique currencies used in transactions
pub fn get_all_currencies(transactions: &[Transaction]) -> HashMap<String, f64> {
    let mut currency_totals: HashMap<String, f64> = HashMap::new();
    
    for transaction in transactions {
        let currency = transaction.currency_iso_4217.clone();
        *currency_totals.entry(currency).or_insert(0.0) += transaction.amount;
    }
    
    currency_totals
}



#[cfg(test)]
mod tests {
    use crate::structs::{Split, SplitType};

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
                    split_type: SplitType::Ratio(Rational::new(numer, denom)),
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
}
