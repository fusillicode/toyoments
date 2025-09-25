use color_eyre::eyre::OptionExt as _;
use csv::ReaderBuilder;
use csv::Trim;

use crate::models::csv_models::Transaction;

mod models;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let tx_file_path = std::env::args()
        .nth(1)
        .ok_or_eyre("no transactions CSV supplied")?;

    let mut tx_file_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(tx_file_path)?;

    for tx_res in tx_file_reader.deserialize::<Transaction>() {
        let Ok(tx) = tx_res else {
            eprintln!("error deserializing transaction, error={tx_res:?}");
            continue;
        };
        dbg!(&tx);
    }

    Ok(())
}
