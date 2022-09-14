use crate::transaction::{Transaction, TransactionDb, TransactionType};
use csv::WriterBuilder;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;

// ------------------------------------------------------------------------------------------------
// -------------------------------- CLIENT DB STRUCT ----------------------------------------------
// ------------------------------------------------------------------------------------------------

// Wrapper struct for client database (hashmap) to avoid exposure to internal hashmap api.
pub struct ClientDb {
    db: HashMap<u16, Client>,
}

// Client struct with renamed fields for clarity. All f64 fields custom serialised to ensure 4.d.p precision.
#[derive(Serialize, Debug)]
pub struct Client {
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(serialize_with = "round_serialize")]
    available: f64,
    #[serde(serialize_with = "round_serialize")]
    held: f64,
    #[serde(serialize_with = "round_serialize")]
    total: f64,
    locked: bool,
}

// Custom Serialiser to round transaction amount to 4.d.p. Runs on point of serialisation.
fn round_serialize<S>(x: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded_to_precision = (x * 10_000.0).round() / 10_000.0;
    s.serialize_f64(rounded_to_precision)
}

// ------------------------------------------------------------------------------------------------
// ----------------------------------- CLIENT DB ASSOCIATED FUNCTIONS -----------------------------
// ------------------------------------------------------------------------------------------------

impl ClientDb {
    // Naming convention `init` chosen over `new` here as assumption is made that
    // database would exist in real-life scenario and would init associated function
    // would create database connection.
    pub fn init() -> Self {
        ClientDb { db: HashMap::new() }
    }

    // Insert a Client record into the db with id as key
    pub fn insert_client_record(&mut self, client_record: Client) {
        self.db.insert(client_record.client_id, client_record);
    }

    // Get a mutable reference to a client record given an id
    pub fn get_client_record(&mut self, client_id: &u16) -> Option<&mut Client> {
        self.db.get_mut(client_id)
    }

    // Write client database as csv to stdout with headers
    pub fn to_csv_stdout(&self) -> Result<(), Box<dyn Error>> {
        let mut writer = WriterBuilder::new().has_headers(true).from_writer(vec![]);
        for client in self.db.values() {
            writer.serialize(client)?;
        }
        let buf = writer.into_inner()?;
        std::io::stdout().write_all(&buf)?;
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// ----------------------------------- CLIENT ASSOCIATED FUNCTIONS --------------------------------
// ------------------------------------------------------------------------------------------------

impl Client {
    // Create new client with given id. Initialised to 0.0 for all account balance metrics and unlocked.
    pub fn new(client_id: u16) -> Self {
        Client {
            client_id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    // Handler function for type of transaction. Performs respective associated function on the client record.
    // If account is locked then early return as no mutations to the client record should take place.
    pub fn apply_transaction_to_client(
        &mut self,
        transaction: &Transaction,
        transaction_db: &TransactionDb,
    ) {
        if self.locked {
            return;
        }

        match transaction.transaction_type {
            TransactionType::Deposit => self.deposit(transaction.amount),
            TransactionType::Withdrawal => self.withdrawal(transaction.amount),
            TransactionType::Dispute => self.dispute(transaction.transaction_id, transaction_db),
            TransactionType::Resolve => self.resolve(transaction.transaction_id, transaction_db),
            TransactionType::Chargeback => {
                self.chargeback(transaction.transaction_id, transaction_db)
            }
        }
    }

    // Updates client account following deposit.
    // If deposit amount is missing, ignore as a bad transaction and do nothing to client account.
    fn deposit(&mut self, deposit_amount: Option<f64>) {
        if let Some(amount) = deposit_amount {
            self.total += amount;
            self.available += amount;
        }
    }

    // Updates Client account following withdrawal
    // If withdrawal amount is missing, ignore as a bad transaction and do nothing to client account.
    fn withdrawal(&mut self, withdrawal_amount: Option<f64>) {
        if let Some(amount) = withdrawal_amount {
            match amount < self.available {
                true => {
                    self.available -= amount;
                    self.total -= amount;
                }
                false => {}
            }
        }
    }

    // Retrieves original transaction data following a dispute claim.
    // If original transaction data doesn't exist or
    // there is no corresponding amount for the specified transaction then the dispute is ignored.
    fn dispute(&mut self, transaction_id: u32, transaction_db: &TransactionDb) {
        let transaction_data = transaction_db.retrieve_transaction_data(&transaction_id);
        if let Some(tx) = transaction_data {
            match tx.amount {
                Some(value) => {
                    self.available -= value;
                    self.held += value;
                }
                None => {}
            }
        }
    }

    // Retrieves original transaction data following a resolve claim.
    // If original transaction data doesn't exist or
    // there is no corresponding amount for the specified transaction then the resolve is ignored.
    fn resolve(&mut self, transaction_id: u32, transaction_db: &TransactionDb) {
        let transaction_data = transaction_db.retrieve_transaction_data(&transaction_id);
        if let Some(tx) = transaction_data {
            match tx.amount {
                Some(value) => {
                    self.available += value;
                    self.held -= value;
                }
                None => {}
            }
        }
    }

    // Retrieves original transaction data following a chargeback claim.
    // If original transaction data doesn't exist or
    // there is no corresponding amount for the specified transaction then the chargeback is ignored.
    fn chargeback(&mut self, transaction_id: u32, transaction_db: &TransactionDb) {
        let transaction_data = transaction_db.retrieve_transaction_data(&transaction_id);
        if let Some(tx) = transaction_data {
            match tx.amount {
                Some(value) => {
                    self.held -= value;
                    self.total -= value;
                    self.locked = true
                }
                None => {}
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// --------------------------------------- UNIT TESTS ---------------------------------------------
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction;

    // Helper function to create client and transction databases in test suite.
    fn create_client_transaction_dbs() -> (ClientDb, TransactionDb) {
        let client_db = ClientDb::init();
        let transaction_db = transaction::TransactionDb::init();
        (client_db, transaction_db)
    }

    #[test]
    fn deposit_correctly_credits_account() {
        // Ensure that when a despoist takes place that the correct mutations take place to both available and total funds.
        let (mut client_db, transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;
        let client = Client::new(client_id);
        client_db.insert_client_record(client);

        let deposit_amount = 100_f64;
        let test_desposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(deposit_amount),
        };

        test_desposit.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record.available, deposit_amount);
        assert_eq!(client_record.total, deposit_amount);
    }

    #[test]
    fn withdraw_correctly_removes_balance() {
        // Checks whether after a withdrawal the correct mutations take place to both available and total funds.
        let (mut client_db, transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;
        let (deposit_amount, withdrawal_amount) = (500_f64, 100_f64);

        let test_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(deposit_amount),
        };
        let test_withdrawal = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(withdrawal_amount),
        };
        test_deposit.handle_transaction(&transaction_db, &mut client_db);
        test_withdrawal.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record.total, deposit_amount - withdrawal_amount);
        assert_eq!(client_record.available, deposit_amount - withdrawal_amount)
    }

    #[test]
    fn withdraw_does_nothing_if_not_enough_available() {
        // Tests that client total does not change if a withdrawal is greater than the avaialbe funds.
        let (mut client_db, transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;
        let (deposit_amount, withdrawal_amount) = (100_f64, 500_f64);

        let test_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(deposit_amount),
        };
        let test_withdrawal = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client_id: client_id,
            transaction_id: 2,
            amount: Some(withdrawal_amount),
        };
        test_deposit.handle_transaction(&transaction_db, &mut client_db);
        test_withdrawal.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record_after_withdrawal = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record_after_withdrawal.total, deposit_amount);
    }

    #[test]
    fn dispute_holds_funds() {
        // Tests whether a dispute correctly mutates the held and available balance of a client.
        let (mut client_db, mut transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;
        let deposit_and_disputed_amount = 100_f64;

        let test_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(deposit_and_disputed_amount),
        };
        let test_dispute = Transaction {
            transaction_type: TransactionType::Dispute,
            client_id: client_id,
            transaction_id: 1,
            amount: None,
        };

        test_deposit.handle_transaction(&transaction_db, &mut client_db);
        transaction_db.insert_transaction(test_deposit);
        test_dispute.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record.held, deposit_and_disputed_amount);
        assert_eq!(client_record.available, 0_f64);
        assert_eq!(client_record.total, deposit_and_disputed_amount);
    }

    #[test]
    fn resolve_releases_held_funds() {
        let (mut client_db, mut transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;
        let held_amount = 100_f64;

        let test_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(100_f64),
        };
        let test_dispute = Transaction {
            transaction_type: TransactionType::Dispute,
            client_id: client_id,
            transaction_id: 1,
            amount: None,
        };
        let test_resolution = Transaction {
            transaction_type: TransactionType::Resolve,
            client_id: client_id,
            transaction_id: 1,
            amount: None,
        };

        test_deposit.handle_transaction(&transaction_db, &mut client_db);
        transaction_db.insert_transaction(test_deposit);
        test_dispute.handle_transaction(&transaction_db, &mut client_db);
        test_resolution.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record_after_dispute = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record_after_dispute.available, held_amount);
    }

    #[test]
    fn chargeback_locks_account() {
        let (mut client_db, mut transaction_db) = create_client_transaction_dbs();
        let client_id = 1u16;

        let test_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: client_id,
            transaction_id: 1,
            amount: Some(100.0),
        };
        let test_chargeback = Transaction {
            transaction_type: TransactionType::Chargeback,
            client_id: client_id,
            transaction_id: 1,
            amount: None,
        };

        test_deposit.handle_transaction(&transaction_db, &mut client_db);
        transaction_db.insert_transaction(test_deposit);
        test_chargeback.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record_after_chargeback = client_db.get_client_record(&client_id).unwrap();
        assert_eq!(client_record_after_chargeback.locked, true);
    }

    #[test]
    fn locked_account_does_not_apply_transaction() {
        // Tests that a transaction will not alter a locked account.
        let (mut client_db, transaction_db) = create_client_transaction_dbs();

        let locked_client = Client {
            client_id: 1,
            available: 100.0,
            held: 0.0,
            total: 100.0,
            locked: true,
        };
        client_db.insert_client_record(locked_client);

        let test_transaction = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: 1,
            amount: Some(100.0),
        };

        // Duplicated as unnecessary to derive Copy and Clone on client for non test purposes.
        let original_client_record = Client {
            client_id: 1,
            available: 100.0,
            held: 0.0,
            total: 100.0,
            locked: true,
        };

        test_transaction.handle_transaction(&transaction_db, &mut client_db);
        // Unwrap used here as we can say for certainty that the client record with id=1_u16 exists
        let client_record = client_db
            .get_client_record(&original_client_record.client_id)
            .unwrap();
        assert_eq!(client_record.available, original_client_record.available);
    }

    #[test]
    fn unknown_client_creates_new_record() {
        // Tests to ensure that a new client record is created if a transaction references a client id that does not exist
        let (mut client_db, transaction_db) = create_client_transaction_dbs();
        let test_desposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(1_f64),
        };
        assert!(client_db.db.is_empty());
        test_desposit.handle_transaction(&transaction_db, &mut client_db);
        assert_eq!(client_db.db.len(), 1);
    }
}
