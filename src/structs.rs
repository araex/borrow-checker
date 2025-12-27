use rational::Rational;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use toml::value::Datetime;
use uuid::Uuid;

pub struct AppState {
    pub current_group: Mutex<String>,
    pub current_ledger: Mutex<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Group {
    entities: Vec<Entity>,
}

#[derive(Serialize, Deserialize)]
pub struct Entity {
    id: Uuid,
    display_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Ledger {
    id: Uuid,
    display_name: String,
    participants: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    description: String,
    paid_by_entity: Uuid,
    currency_iso_4217: String,
    amount: f64,
    transaction_datetime_rfc_3339: Datetime,
    split_ratios: Vec<Split>,
}

#[derive(Serialize, Deserialize)]
pub struct Split {
    entity_id: Uuid,
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
        test_data1.push("data/borrow-checker-testdata/ledgers/39C3/019b5b4f-8077-7c4b-89d4-9380c444ee9d.toml");
        


        let foo = read_to_string(test_data1).unwrap();

        let sut: Transaction = toml::from_str(&foo).unwrap();

        assert_eq!(sut.description, "🛫");
    }

    #[test]
    fn test_parse_group() {
        let mut test_data = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data.push("data/borrow-checker-testdata/group.toml");

        let content = read_to_string(test_data).unwrap();
        let sut: Group = toml::from_str(&content).unwrap();

        assert_eq!(sut.entities.len(), 3);
        assert_eq!(sut.entities[0].display_name, "Araex");
        assert_eq!(sut.entities[1].display_name, "Wüstenschiff");
        assert_eq!(sut.entities[2].display_name, "flakmonkey");
    }

    #[test]
    fn test_parse_ledger() {
        let mut test_data = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data.push("data/borrow-checker-testdata/ledgers/39C3/.ledger.toml");

        let content = read_to_string(test_data).unwrap();
        let sut: Ledger = toml::from_str(&content).unwrap();

        assert_eq!(sut.display_name, "39C3");
        assert_eq!(sut.participants.len(), 3);
    }

    #[test]
    fn test_parse_entity() {
        let mut test_data = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data.push("data/borrow-checker-testdata/group.toml");

        let content = read_to_string(test_data).unwrap();
        let group: Group = toml::from_str(&content).unwrap();

        assert_eq!(group.entities[0].id.to_string(), "c8744a29-7ed0-447a-af5a-51e4ad291d1d");
        assert_eq!(group.entities[1].id.to_string(), "3abaaf40-a35a-488d-8ef2-0184c8c5f3c3");
        assert_eq!(group.entities[2].id.to_string(), "92c0a0fc-aa86-4922-ab1f-7b9326720177");
    }
}
