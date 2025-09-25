use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClientId(u16);

#[derive(Debug, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct TransactionId(u32);

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

impl Transaction {
    pub fn id(&self) -> TransactionId {
        match self {
            Transaction::Deposit(Deposit { id, .. })
            | Transaction::Withdrawal(Withdrawal { id, .. })
            | Transaction::Dispute(Dispute { id, .. })
            | Transaction::Resolve(Resolve { id, .. })
            | Transaction::Chargeback(Chargeback { id, .. }) => *id,
        }
    }

    pub fn client_id(&self) -> ClientId {
        match self {
            Transaction::Deposit(Deposit { client_id, .. })
            | Transaction::Withdrawal(Withdrawal { client_id, .. })
            | Transaction::Dispute(Dispute { client_id, .. })
            | Transaction::Resolve(Resolve { client_id, .. })
            | Transaction::Chargeback(Chargeback { client_id, .. }) => *client_id,
        }
    }
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CsvRow {
            client: ClientId,
            tx: TransactionId,
            r#type: String,
            amount: Option<PositiveAmount>,
        }

        let row = CsvRow::deserialize(deserializer)?;

        let tx = match row.r#type.as_str() {
            "deposit" => row.amount.map_or_else(
                || Err(serde::de::Error::missing_field("amount")),
                |amount| {
                    Ok(Transaction::Deposit(Deposit {
                        client_id: row.client,
                        id: row.tx,
                        amount,
                    }))
                },
            ),
            "withdrawal" => row.amount.map_or_else(
                || Err(serde::de::Error::missing_field("amount")),
                |amount| {
                    Ok(Transaction::Withdrawal(Withdrawal {
                        client_id: row.client,
                        id: row.tx,
                        amount,
                    }))
                },
            ),
            "dispute" => Ok(Transaction::Dispute(Dispute {
                client_id: row.client,
                id: row.tx,
            })),
            "resolve" => Ok(Transaction::Resolve(Resolve {
                client_id: row.client,
                id: row.tx,
            })),
            "chargeback" => Ok(Transaction::Chargeback(Chargeback {
                client_id: row.client,
                id: row.tx,
            })),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["deposit", "withdrawal", "dispute", "resolve", "chargeback"],
            )),
        }?;

        Ok(tx)
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Deposit {
    client_id: ClientId,
    id: TransactionId,
    amount: PositiveAmount,
}

impl Deposit {
    pub fn amount(&self) -> PositiveAmount {
        self.amount
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Withdrawal {
    client_id: ClientId,
    id: TransactionId,
    amount: PositiveAmount,
}

impl Withdrawal {
    pub fn amount(&self) -> PositiveAmount {
        self.amount
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Dispute {
    client_id: ClientId,
    id: TransactionId,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Resolve {
    client_id: ClientId,
    id: TransactionId,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Chargeback {
    client_id: ClientId,
    id: TransactionId,
}

/// This permits to avoid checks on negative amount while processing transactions.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct PositiveAmount(Decimal);

impl PositiveAmount {
    pub fn as_inner(&self) -> Decimal {
        self.0
    }
}

impl<'de> Deserialize<'de> for PositiveAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let decimal = <Decimal as serde::Deserialize>::deserialize(deserializer)?;

        if decimal.is_sign_negative() {
            return Err(serde::de::Error::custom("Decimal must be positive"));
        }

        Ok(PositiveAmount(decimal))
    }
}

#[cfg(test)]
mod tests {
    use csv::Trim;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    use super::*;

    #[rstest]
    #[case(
        "deposit,1,10,1.25",
        Transaction::Deposit(Deposit {
            client_id: ClientId(1),
            id: TransactionId(10),
            amount: PositiveAmount(Decimal::from_str("1.25").unwrap()),
        })
    )]
    #[case(
        "withdrawal,2,11,2.00",
        Transaction::Withdrawal(Withdrawal {
            client_id: ClientId(2),
            id: TransactionId(11),
            amount: PositiveAmount(Decimal::from_str("2.00").unwrap()),
        })
    )]
    #[case(
        "dispute,3,12,",
        Transaction::Dispute(Dispute {
            client_id: ClientId(3),
            id: TransactionId(12),
        })
    )]
    #[case(
        "resolve,4,13,",
        Transaction::Resolve(Resolve {
            client_id: ClientId(4),
            id: TransactionId(13),
        })
    )]
    #[case(
        "chargeback,5,14,",
        Transaction::Chargeback(Chargeback {
            client_id: ClientId(5),
            id: TransactionId(14)
        }))
    ]
    fn deserialize_transaction_returns_the_expected_transactions(
        #[case] csv_row: &str,
        #[case] expected: Transaction,
    ) {
        let txs = deserialize_csv_rows(csv_row).unwrap();
        assert_eq!([expected], txs.as_slice());
    }

    #[rstest]
    #[case("deposit,6,15,", "missing field `amount`")]
    #[case("deposit,7,16,-5.00", "Decimal must be positive")]
    #[case("withdrawal,9,18,", "missing field `amount`")]
    #[case("withdrawal,10,19,-7.50", "Decimal must be positive")]
    #[case(
        "foobar,8,17,1.00",
        "unknown variant `foobar`, expected one of `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`"
    )]
    fn deserialize_transaction_returns_the_expected_error(
        #[case] csv_row: &str,
        #[case] expected_substr: &str,
    ) {
        assert2::let_assert!(Err(error) = deserialize_csv_rows(csv_row));
        assert!(
            error.to_string().contains(expected_substr),
            "error={:?} does not contain expected={expected_substr}'",
            error
        );
    }

    fn deserialize_csv_rows(row: &str) -> Result<Vec<Transaction>, csv::Error> {
        let data = format!("type,client,tx,amount\n{row}");
        let mut rdr = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(data.as_bytes());
        let mut out = Vec::new();
        for rec in rdr.deserialize::<Transaction>() {
            out.push(rec?);
        }
        Ok(out)
    }
}
