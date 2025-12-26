use rational::Rational;
use serde::{Deserialize, Serialize};
use toml::value::Datetime;

#[derive(Serialize, Deserialize)]
struct Transaction {
    description: String,
    paid_by_entity: String,
    currency_iso_4217: String,
    amount: f64,
    transaction_datetime_rfc_3339: Datetime,
    split_ratios: Vec<Split>,
}

#[derive(Serialize, Deserialize)]
struct Split {
    entity_id: String,
    ratio: Rational,
}

#[cfg(test)]
mod tests {
    use std::{
        fs::read_to_string,
        path::{Path, PathBuf},
    };

    use super::*;

    #[test]
    fn test_parse_transaction_description() {
        let mut test_data1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data1.push(
            "data/borrow-checker-testdata/ledgers/39C3/019b5b4f-8077-7c4b-89d4-9380c444ee9d.toml",
        );

        let foo = read_to_string(test_data1).unwrap();

        let sut: Transaction = toml::from_str(&foo).unwrap();

        assert_eq!(sut.description, "🛫");
    }
}
