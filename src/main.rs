mod cli_args;
mod client;
mod transaction;

use clap::Parser;
use cli_args::CliArgs;
use client::ClientDb;
use transaction::TransactionDb;

fn main() {
    // Read args supplied to binary. CLAP throws error if no argument is supplied.
    // Explains that the transaction file argument is required.
    let args: CliArgs = cli_args::CliArgs::parse();

    // Create csv reader from supplied path to binary. Panics if invalid file.
    let tx_reader = args.create_tx_reader();

    // Create Transaction Database for storing desposit and withdrawals in case of dispute|resolve|chargeback.
    // In a real-life scenario it is assumed that the associated function init would initiate a database connection.
    let mut transaction_db = TransactionDb::init();

    // Initiate Client Database for creating/mutating client records.
    // In a real-life scenario it is assumed that the associated function init would initiate a database connection.
    let mut client_db = ClientDb::init();

    // Apply Transactions to Client Database or exit on error.
    if let Err(err) =
        transaction::apply_transactions(tx_reader, &mut transaction_db, &mut client_db)
    {
        println!("Error applying transactions to client database: {}", err);
        std::process::exit(1)
    }

    // Send Client Records csv formatted to stdout or exit on error.
    if let Err(err) = client_db.to_csv_stdout() {
        println!("Error sending client database to stdout: {}", err);
        std::process::exit(1)
    }
}
