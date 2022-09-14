use clap::Parser;
use csv::{Reader, ReaderBuilder, Trim};
use std::fs::File;

/// Program to read transactions from a csv file and apply valid transactions to client database.
#[derive(Parser, Debug)]
pub struct CliArgs {
    /// Relative path to transaction csv file.
    #[clap(value_parser)]
    transaction_file_path: String,
}

// Build the csv reader from the path supplied to the binary.
// Panics if specified filename is invalid.
impl CliArgs {
    pub fn create_tx_reader(self) -> Reader<File> {
        ReaderBuilder::new()
            .trim(Trim::All)
            .from_path(self.transaction_file_path)
            .expect("Failed to initalise CSV reader. Please ensure specified path is correct")
    }
}
