# Transaction Payment Handler Engine.


### Introduction

This CLI-based application is a toy transaction engine for handling transactions and applying valid transactions to a client's account.

### Design

As a mock transaction engine, two databases are initiated before handling transactions. One to store `Client` records, and one for `Transaction` records.

In a real-world example these database initiations would connect to an actual database, rather than being implemented with `HashMap`.

As the transactions are read, the transaction is handled and depending on the type of transaction, the relevant effect on the client's account is made (unless the client's account is locked).

All transaction amounts are deserialised with 4 decimal place precision, and likewise all client account metrics are serialised to the same precision.

### Input

The input to the application (as a CLI argument) is the relative path to a CSV file of transactions with headers:

`type, client, tx, amount`

`type` is the type of transaction, one of:

`Deposit, Withdrawal, Dispute, Resolve, Chargeback`

`client` is a Client id.

`tx` is a Tansaction id.

`amount` is the amount of the transaction.

### Output

The application outputs the Client records after the inputted list of transactions have been applied to their accounts. This output is written to stdout (CSV formatted) with headers:

`client, available, held, total, locked`

### Usage

Example usage of the application :

`cargo run -- -file_path.csv > clients.csv` (Debug Mode)

`cargo run -r -- -file_path.csv > clients.csv` (Release Mode)


### Testing

Unit-Tests are written at the bottom of the three modules: `cli_args, transaction, client`

12 Tests have been written to ensure the following:

    1.  Invalid path supplied to the binary causes it to panic.
    2.  Valid path supplied to the binary successfully creates a CSV reader.
    3.  Deposits and Withdrawal Transactions are added to the transaction database.
    4.  Disputes, Resolutions, and Charebacks are not added to the transaction database.
    5.  Deposits correctly credits a client's account.
    6.  Withdrawals correctly debits a client's account (when balance is sufficent).
    7.  Withdrawals are ignored if the amount exceeds a clients available balance.
    8.  Disputes put the disputed transaction's amount on hold and decrease available funds by the same amount.
    9.  Resolutions release the held funds and correctly credit the client's available balance.
    10. Chargebacks freeze the client's account.
    11. If a client is unknown, a new record is created for them and stored in the client database.
    12. Transactions relating to locked accounts will have no effect.
