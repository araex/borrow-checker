use rational::Rational;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use toml::value::Datetime;
use uuid::Uuid;

pub struct AppState {
    pub group: Mutex<Group>,
    pub ledgers: Mutex<Vec<LedgerWithTransactions>>,
    pub current_ledger_id: Mutex<Option<Uuid>>,
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
    pub entities: Vec<Entity>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Entity {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Ledger {
    pub id: Uuid,
    pub display_name: String,
    pub participants: Vec<Uuid>,
}

#[derive(Clone)]
pub struct LedgerWithTransactions {
    pub ledger: Ledger,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub description: String,
    pub paid_by_entity: Uuid,
    pub currency_iso_4217: String,
    pub amount: f64,
    pub transaction_datetime_rfc_3339: Datetime,
    pub split_ratios: Vec<Split>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Split {
    pub entity_id: Uuid,
    pub ratio: Rational,
}

#[cfg(test)]
mod tests {
    
    use std::{
        fs::read_to_string,
        path::{Path, PathBuf},
    };
    
    use test_context::{test_context, TestContext};

    use super::*;
    struct TestTransactions {
        flight: Transaction,
        ticket: Transaction,   
    }

    impl TestContext for TestTransactions {
        fn setup() -> Self {
            let ticket:Transaction = toml::from_str(&read_toml("data/borrow-checker-testdata/ledgers/39C3/019b5b3b-25e7-7e53-a0b6-0af3afde297c.toml").unwrap()).unwrap();
            let flight:Transaction = toml::from_str(&read_toml("data/borrow-checker-testdata/ledgers/39C3/019b5b4f-8077-7c4b-89d4-9380c444ee9d.toml").unwrap()).unwrap();

            TestTransactions { flight, ticket }
        }
    }

    fn read_toml(toml_relative_path: &str) -> Result<String, std::io::Error> {
        let mut test_data_full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_full_path.push(toml_relative_path);
       
        read_to_string(test_data_full_path)
    }

    #[test_context(TestTransactions)]
    #[test]
    fn test_transaction_description_flight(sut: &mut TestTransactions) {
        assert_eq!(sut.flight.description, "🛫");
    }

    #[test_context(TestTransactions)]
    #[test]
    fn test_transaction_description_ticket(sut: &mut TestTransactions) {
        assert_eq!(sut.ticket.description, "🎫🍖🏰");
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
