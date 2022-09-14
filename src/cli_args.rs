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

#[cfg(test)]
mod tests {
    use super::*;

    // Create reader from path
    fn create_tx_reader(path: String) -> Reader<File> {
        ReaderBuilder::new()
            .trim(Trim::All)
            .from_path(path)
            .expect("Failed to initalise CSV reader. Please ensure specified path is correct")
    }

    #[test]
    #[should_panic]
    fn invalid_path_panics() {
        // Make sure that an invalid path causes the csv reader to panic
        let path = "not_a_valid_path.csv".to_string();
        let _ = create_tx_reader(path);
    }

    #[test]
    fn valid_path_creates_reader() -> Result<(), Box<dyn std::error::Error>> {
        // Create temp directory to test that csv reader reads valid path correctly
        let dir = tempfile::tempdir()?;
        let file_path = dir.path().join("temp_csv_file.csv");
        let _ = File::create(&file_path)?;
        let _ = create_tx_reader(file_path.as_path().display().to_string());
        Ok(())
    }
}
