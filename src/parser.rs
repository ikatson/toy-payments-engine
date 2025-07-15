use crate::{
    Error,
    accounts::{ClientId, Transaction, TransactionId, TransactionKind},
    amount::Amount,
};

#[derive(Debug, Eq, PartialEq)]
pub struct Row {
    pub client_id: ClientId,
    pub transaction: Transaction,
}

impl Row {
    /// Parse a CSV row assuming header "type, client, tx, amount"
    pub fn parse(buf: &[u8]) -> Result<Self, crate::Error> {
        let mut columns =
            memchr::memchr_iter(b',', buf)
                .chain(Some(buf.len()))
                .scan(0usize, |start, end| {
                    let column = buf[*start..end].trim_ascii();
                    *start = end + 1;
                    Some(column)
                });
        let ttype = columns.next().ok_or(Error::CsvMissingColumn)?;
        let client_id = columns.next().ok_or(Error::CsvMissingColumn)?;
        let tx_id = columns.next().ok_or(Error::CsvMissingColumn)?;
        let amount = columns.next().ok_or(Error::CsvMissingColumn)?;

        let ttype = match ttype {
            b"deposit" => TransactionKind::Deposit,
            b"withdrawal" => TransactionKind::Withdrawal,
            b"dispute" => TransactionKind::Dispute,
            b"resolve" => TransactionKind::Resolve,
            b"chargeback" => TransactionKind::Chargeback,
            _ => return Err(Error::CsvUnknownTransactionType),
        };

        let client_id: ClientId = atoi::atoi(client_id).ok_or(Error::CsvInvalidClientId)?;
        let tx_id: TransactionId = atoi::atoi(tx_id).ok_or(Error::CsvInvalidTxId)?;

        let amount = if ttype.has_amount() {
            Amount::parse(amount).ok_or(Error::CsvInvalidAmount)?
        } else if !amount.is_empty() {
            return Err(Error::CsvUnexpectedAmount);
        } else {
            Amount::zero()
        };
        Ok(Row {
            client_id,
            transaction: Transaction {
                kind: ttype,
                id: tx_id,
                amount,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, accounts::Transaction, amount::Amount, parser::Row};

    #[test]
    fn test_parse() {
        assert_eq!(
            Row::parse(b"deposit, 1, 1, 1.0").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Deposit,
                    id: 1,
                    amount: Amount::parse(b"1.0").unwrap()
                }
            }
        );

        // extra whitespace
        assert_eq!(
            Row::parse(b"    deposit   ,  1   , 1   , 1.0   ").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Deposit,
                    id: 1,
                    amount: Amount::parse(b"1.0").unwrap()
                }
            }
        );

        assert_eq!(
            Row::parse(b"withdrawal, 1, 1, 1.0").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Withdrawal,
                    id: 1,
                    amount: Amount::parse(b"1.0").unwrap()
                }
            }
        );

        assert_eq!(
            Row::parse(b"dispute, 1, 1, ").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Dispute,
                    id: 1,
                    amount: Amount::zero()
                }
            }
        );
        assert_eq!(
            Row::parse(b"resolve, 1, 1, ").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Resolve,
                    id: 1,
                    amount: Amount::zero()
                }
            }
        );
        assert_eq!(
            Row::parse(b"chargeback, 1, 1, ").unwrap(),
            Row {
                client_id: 1,
                transaction: Transaction {
                    kind: crate::accounts::TransactionKind::Chargeback,
                    id: 1,
                    amount: Amount::zero()
                }
            }
        );

        // invalid
        assert!(matches!(
            Row::parse(b"").unwrap_err(),
            Error::CsvMissingColumn
        ));
        assert!(matches!(
            Row::parse(b",,").unwrap_err(),
            Error::CsvMissingColumn
        ));
        assert!(matches!(
            Row::parse(b",,,,,,").unwrap_err(),
            Error::CsvUnknownTransactionType
        ));
        assert!(matches!(
            Row::parse(b",,,").unwrap_err(),
            Error::CsvUnknownTransactionType
        ));
        assert!(matches!(
            Row::parse(b"deposit,1,1").unwrap_err(),
            Error::CsvMissingColumn
        ));
        assert!(matches!(
            Row::parse(b"deposit,1,1,").unwrap_err(),
            Error::CsvInvalidAmount
        ));
        assert!(matches!(
            Row::parse(b"resolve,1,1,1.0").unwrap_err(),
            Error::CsvUnexpectedAmount
        ));
        assert!(matches!(
            Row::parse(b"foo, 1, 1, 1.0").unwrap_err(),
            Error::CsvUnknownTransactionType
        ));
        assert!(matches!(
            Row::parse(b"withdrawal, foo, 1, 1.0").unwrap_err(),
            Error::CsvInvalidClientId
        ));
        assert!(matches!(
            Row::parse(b"withdrawal, 1, foo, 1.0").unwrap_err(),
            Error::CsvInvalidTxId
        ));
    }
}
