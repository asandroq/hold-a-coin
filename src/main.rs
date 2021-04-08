
mod account;

use csv;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use serde::Deserialize;

use account::model::{Amount, ClientId, Transaction, Tx};
use account::service::AccountStorage;


#[derive(Debug, Deserialize)]
struct TransactionRow {
    #[serde(rename(deserialize = "type"))]
    kind: String,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

fn process_input(storage: &mut AccountStorage, source_csv: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(&source_csv)?;
    for result in reader.deserialize() {
        let row: TransactionRow = result?;
        eprintln!("{:?}", row);
        let client_id = ClientId::from(row.client);
        let tx_id = Tx::from(row.tx);
        let tx = match row.kind.as_str() {
            "deposit" => {
                if let Some(amount) = row.amount {
                    let amount = Amount::try_from(amount)?;
                    Ok(Transaction::Deposit(tx_id, amount))
                } else {
                    Err("Deposit transaction is missing an amount")
                }
            },
            "withdrawal" => {
                if let Some(amount) = row.amount {
                    let amount = Amount::try_from(amount)?;
                    Ok(Transaction::Withdrawal(tx_id, amount))
                } else {
                    Err("Withdrawal transaction is missing an amount")
                }
            },
            "dispute" => {
                Ok(Transaction::Dispute(tx_id))
            },
            "resolve" => {
                Ok(Transaction::Resolve(tx_id))
            },
            "chargeback" => {
                Ok(Transaction::Chargeback(tx_id))
            },
            _ => {
                Err("Unknown transaction type")
            }
        }?;

        match storage.apply_transaction(&client_id, tx) {
            Err(err) => eprintln!("Could not process transaction {}: {}", row.tx, err),
            _ => (),
        }
    }

    Ok(())
}

fn print_output(storage: &AccountStorage) -> Result<(), Box<dyn Error>> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());

    writer.write_record(&["client", "available", "held", "total", "locked"])?;
    for (client_id, account) in storage.iter() {
        let total_amt = account.available.add(account.held)?;
        let client = format!("{}", client_id);
        let available = format!("{}", account.available);
        let held = format!("{}", account.held);
        let locked = format!("{}", account.locked);
        let total = format!("{}", total_amt);
        writer.write_record(&[client, available, held, total, locked])?;
    }
    writer.flush()?;

    Ok(())
}

fn main() {
    let mut storage = AccountStorage::new();

    let mut args = env::args();
    if args.len() == 2 {
        let source_csv = args.nth(1).unwrap();
        if let Err(err) = process_input(&mut storage, &source_csv) {
            eprintln!("Could not process input file: {}", err);
        }

        if let Err(err) = print_output(&storage) {
            eprintln!("Could not generate CSV report: {}", err)
        }
    }
}
