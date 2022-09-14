use csv::Reader;
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, error::Error, fs::File};

use crate::client;

// ------------------------------------------------------------------------------------------------
// --------------------------------- APPLY TRANSACTIONS FUNCION -----------------------------------
// ------------------------------------------------------------------------------------------------

// Iterates over rows of transactions from csv reader.
// Handles each transaction with respect to the Client and Transaction Databases.
pub fn apply_transactions(
    mut rdr: Reader<File>,
    transaction_db: &mut TransactionDb,
    client_db: &mut client::ClientDb,
) -> Result<(), Box<dyn Error>> {
    for row in rdr.deserialize() {
        let transaction: Transaction = row?;
        transaction.handle_transaction(transaction_db, client_db);
        transaction_db.insert_transaction(transaction) // Only adds transaction if of type deposit/withdrawal.
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// -------------------------------- TRANSACTION DB STRUCT -----------------------------------------
// ------------------------------------------------------------------------------------------------

// Wrapper struct transaction database (hashmap) to avoid exposure to internal hashmap api.
pub struct TransactionDb {
    db: HashMap<u32, Transaction>,
}

// Transaction type enum as finite list of options. Avoids matching transaction type as string.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// Transaction Struct with renamed fields for clarity and to avoid using `type` keyword.
#[derive(Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub transaction_id: u32,
    #[serde(deserialize_with = "round_deserialise")]
    pub amount: Option<f64>,
}

// Custom Deserialiser to round transaction amount to 4.d.p. Runs on point of deserialising csv.
fn round_deserialise<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let x: Result<f64, _> = Deserialize::deserialize(deserializer);
    // If x is error then the field was None in the CSV as empty string cannot be deserialised.
    // Therefore we return None as there is no amount to round.
    match x {
        Ok(value) => {
            let rounded_to_precision = (value * 10_000.0).round() / 10_000.0;
            Ok(Some(rounded_to_precision))
        }
        Err(_) => Ok(None),
    }
}

// ------------------------------------------------------------------------------------------------
// ------------------------------ TRANSACTION DB ASSOCIATED FUNCTIONS -----------------------------
// ------------------------------------------------------------------------------------------------

impl TransactionDb {
    // Naming convention `init` chosen over `new` here as assumption is made that
    // database would exist in real-life scenario and would init associated function
    // would create database connection.
    pub fn init() -> Self {
        Self { db: HashMap::new() }
    }

    // Insert transaction if of type deposit or withdrawal.
    pub fn insert_transaction(&mut self, transaction: Transaction) {
        match transaction.transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                self.db.insert(transaction.transaction_id, transaction);
            }
            _ => {}
        }
    }
    // Retrieves immutable reference to a transaction from the database.
    pub fn retrieve_transaction_data(&self, transaction_id: &u32) -> Option<&Transaction> {
        self.db.get(transaction_id)
    }
}

// ------------------------------------------------------------------------------------------------
// ------------------------------ TRANSACTION ASSOCIATED FUNCTIONS --------------------------------
// ------------------------------------------------------------------------------------------------

impl Transaction {
    // Applies transaction to a client record
    pub fn handle_transaction(
        &self,
        transaction_db: &TransactionDb,
        client_db: &mut client::ClientDb,
    ) {
        let client_record = client_db.get_client_record(&self.client_id);

        // If record exists deref and apply transaction to the record.
        // If no record, create client record, apply transaction to the record, and store.
        match client_record {
            Some(record) => {
                (*record).apply_transaction_to_client(self, transaction_db);
            }
            None => {
                let mut new_client_record = client::Client::new(self.client_id);
                new_client_record.apply_transaction_to_client(self, transaction_db);
                client_db.insert_client_record(new_client_record);
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

    #[test]
    fn dispute_resolve_chargeback_not_added_to_db() {
        // Make sure disuptes, resolutions, and chargebacks are not added to the transaction_db.
        let mut transaction_db = TransactionDb::init();
        let test_transactions = vec![
            Transaction {
                transaction_type: TransactionType::Dispute,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                transaction_type: TransactionType::Resolve,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
            Transaction {
                transaction_type: TransactionType::Chargeback,
                client_id: 1,
                transaction_id: 1,
                amount: None,
            },
        ];
        for transaction in test_transactions {
            transaction_db.insert_transaction(transaction);
        }
        assert!(transaction_db.db.is_empty())
    }

    #[test]
    fn desposit_withdrawal_added_to_db() {
        // Make sure deposits and withdrawals are added to the transaction db.
        let mut transaction_db = TransactionDb::init();
        let test_transactions = vec![
            Transaction {
                transaction_type: TransactionType::Deposit,
                client_id: 1,
                transaction_id: 1,
                amount: Some(10.0),
            },
            Transaction {
                transaction_type: TransactionType::Withdrawal,
                client_id: 1,
                transaction_id: 2,
                amount: Some(5.0),
            },
        ];
        let number_of_transactions_to_be_inserted = test_transactions.len();
        for transaction in test_transactions {
            transaction_db.insert_transaction(transaction);
        }
        assert!(transaction_db.db.len() == number_of_transactions_to_be_inserted)
    }
}
