use color_eyre::eyre::bail;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd, parse_display::Display)]
pub struct ClientId(pub u16);

#[derive(Debug, Deserialize, Copy, Clone, Hash, PartialEq, Eq, parse_display::Display)]
pub struct TransactionId(pub u32);

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Transaction {
    #[display("{0}")]
    Deposit(Deposit),
    #[display("{0}")]
    Withdrawal(Withdrawal),
    #[display("{0}")]
    Dispute(Dispute),
    #[display("{0}")]
    Resolve(Resolve),
    #[display("{0}")]
    Chargeback(Chargeback),
}

impl Transaction {
    pub const fn id(&self) -> TransactionId {
        match self {
            Self::Deposit(Deposit { id, .. })
            | Self::Withdrawal(Withdrawal { id, .. })
            | Self::Dispute(Dispute { id, .. })
            | Self::Resolve(Resolve { id, .. })
            | Self::Chargeback(Chargeback { id, .. }) => *id,
        }
    }

    pub const fn client_id(&self) -> ClientId {
        match self {
            Self::Deposit(Deposit { client_id, .. })
            | Self::Withdrawal(Withdrawal { client_id, .. })
            | Self::Dispute(Dispute { client_id, .. })
            | Self::Resolve(Resolve { client_id, .. })
            | Self::Chargeback(Chargeback { client_id, .. }) => *client_id,
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
                    Ok(Self::Deposit(Deposit {
                        client_id: row.client,
                        id: row.tx,
                        amount,
                    }))
                },
            ),
            "withdrawal" => row.amount.map_or_else(
                || Err(serde::de::Error::missing_field("amount")),
                |amount| {
                    Ok(Self::Withdrawal(Withdrawal {
                        client_id: row.client,
                        id: row.tx,
                        amount,
                    }))
                },
            ),
            "dispute" => Ok(Self::Dispute(Dispute {
                client_id: row.client,
                id: row.tx,
            })),
            "resolve" => Ok(Self::Resolve(Resolve {
                client_id: row.client,
                id: row.tx,
            })),
            "chargeback" => Ok(Self::Chargeback(Chargeback {
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

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[display("tx=(deposit id={id} client_id={client_id} amount={amount})")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Deposit {
    pub client_id: ClientId,
    pub id: TransactionId,
    pub amount: PositiveAmount,
}

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[display("tx=(withdrawal id={id} client_id={client_id} amount={amount})")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Withdrawal {
    pub client_id: ClientId,
    pub id: TransactionId,
    pub amount: PositiveAmount,
}

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[display("tx=(dispute id={id} client_id={client_id})")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Dispute {
    pub client_id: ClientId,
    pub id: TransactionId,
}

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[display("tx=(resolve id={id} client_id={client_id})")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Resolve {
    pub client_id: ClientId,
    pub id: TransactionId,
}

#[derive(Debug, Clone, Copy, parse_display::Display)]
#[display("tx=(chargeback id={id} client_id={client_id})")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Chargeback {
    pub client_id: ClientId,
    pub id: TransactionId,
}

/// This permits to avoid checks on negative amount while handling transactions.
#[derive(Debug, Copy, Clone, parse_display::Display)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct PositiveAmount(Decimal);

impl TryFrom<Decimal> for PositiveAmount {
    type Error = color_eyre::Report;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value.is_sign_negative() {
            bail!("Decimal must be positive value={value:?}");
        }
        Ok(Self(value))
    }
}

impl PositiveAmount {
    pub const fn as_inner(&self) -> Decimal {
        self.0
    }
}

impl<'de> Deserialize<'de> for PositiveAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let decimal = <Decimal as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_from(decimal).map_err(|error| serde::de::Error::custom(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use csv::Trim;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use rust_decimal::Decimal;

    use super::*;

    #[rstest]
    #[case(
        "deposit,20,30,1.2345",
        Transaction::Deposit(Deposit {
            client_id: ClientId(20),
            id: TransactionId(30),
            amount: PositiveAmount(Decimal::from_str("1.2345").unwrap()),
        })
    )]
    #[case(
        "withdrawal,21,31,2.0001",
        Transaction::Withdrawal(Withdrawal {
            client_id: ClientId(21),
            id: TransactionId(31),
            amount: PositiveAmount(Decimal::from_str("2.0001").unwrap()),
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
    fn deserialize_transaction_returns_the_expected_transactions(#[case] csv_row: &str, #[case] expected: Transaction) {
        assert2::let_assert!(Ok(txs) = deserialize_csv_rows(csv_row));
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
    fn deserialize_transaction_returns_the_expected_error(#[case] csv_row: &str, #[case] expected_substr: &str) {
        assert2::let_assert!(Err(error) = deserialize_csv_rows(csv_row));
        assert!(
            error.to_string().contains(expected_substr),
            "error={error:?} does not contain expected={expected_substr}'",
        );
    }

    fn deserialize_csv_rows(row: &str) -> Result<Vec<Transaction>, csv::Error> {
        let data = format!("type,client,tx,amount\n{row}");
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(data.as_bytes());
        let mut out = Vec::new();
        for rec in rdr.deserialize::<Transaction>() {
            out.push(rec?);
        }
        Ok(out)
    }
}
