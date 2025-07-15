use std::collections::{HashMap, hash_map::Entry};

use crate::{Error, amount::Amount};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TransactionKind {
    pub fn has_amount(&self) -> bool {
        matches!(self, TransactionKind::Deposit | TransactionKind::Withdrawal)
    }
}

pub type TransactionId = u32;
pub type ClientId = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub kind: TransactionKind,
    pub id: TransactionId,
    pub amount: Amount,
}

struct Deposit {
    transaction_id: TransactionId,
    amount: Amount,
    is_disputed: bool,
}

#[derive(Default)]
pub struct Account {
    // We only store deposits as only deposits can be disputed (this isn't clearly specified but can
    // be deduced from the description of dispute section).
    // Stored in TXID order for binary search.
    //
    // We could store other transactions to detect duplicate transaction IDs. However for the toy implementation
    // this would be overkill and would decrease perf just to detect one edge case.
    deposits: Vec<Deposit>,
    total: Amount,
    // Held can be greater than total, in case there's a transaction under dispute
    held: Amount,
    frozen: bool,
}

impl Account {
    pub fn available_for_withdrawal(&self) -> Amount {
        if self.frozen {
            return Amount::zero();
        }
        self.total.checked_sub(self.held).unwrap_or_default()
    }

    pub fn total(&self) -> Amount {
        self.total
    }

    pub fn held(&self) -> Amount {
        self.held
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    fn find_deposit_id(&self, tid: TransactionId) -> Result<usize, crate::Error> {
        let deposit_idx = self
            .deposits
            .binary_search_by_key(&tid, |d| d.transaction_id)
            .map_err(|_| Error::TransactionNotFound)?;
        Ok(deposit_idx)
    }

    /// Process the transaction and update the account if successful.
    /// If an error is returned, no modification was made to internal state.
    pub fn process(&mut self, t: Transaction) -> Result<(), crate::Error> {
        if self.frozen {
            return Err(Error::AccountFrozen);
        }

        match t.kind {
            TransactionKind::Deposit => {
                let insert_at = match self
                    .deposits
                    .binary_search_by_key(&t.id, |t| t.transaction_id)
                {
                    Ok(_) => return Err(Error::DuplicateTransactionId),
                    Err(insert_at) => insert_at,
                };
                self.total = self
                    .total
                    .checked_add(t.amount)
                    .ok_or(Error::DepositOverflow)?;
                self.deposits.insert(
                    insert_at,
                    Deposit {
                        transaction_id: t.id,
                        amount: t.amount,
                        is_disputed: false,
                    },
                );
                Ok(())
            }
            TransactionKind::Withdrawal => {
                self.available_for_withdrawal()
                    .checked_sub(t.amount)
                    .ok_or(Error::WithdrawOverflow)?;
                // If this unwrap fails it's a bug.
                self.total = self.total.checked_sub(t.amount).unwrap();
                Ok(())
            }
            TransactionKind::Dispute => {
                let did = self.find_deposit_id(t.id)?;
                if self.deposits[did].is_disputed {
                    return Err(Error::DuplicateDispute);
                }
                self.held = self
                    .held
                    .checked_add(self.deposits[did].amount)
                    .ok_or(Error::HeldOverflow)?;
                self.deposits[did].is_disputed = true;
                Ok(())
            }
            TransactionKind::Resolve => {
                let did = self.find_deposit_id(t.id)?;
                if !self.deposits[did].is_disputed {
                    return Err(Error::ResolveNotDisputed);
                }
                // If this fails it's a bug
                self.held = self.held.checked_sub(self.deposits[did].amount).unwrap();
                self.deposits[did].is_disputed = false;
                Ok(())
            }
            TransactionKind::Chargeback => {
                let did = self.find_deposit_id(t.id)?;
                if !self.deposits[did].is_disputed {
                    return Err(Error::ChargebackNotDisputed);
                }
                self.held = self.held.checked_sub(self.deposits[did].amount).unwrap();
                // If the charged back transaction is more than available funds, set them to 0.
                // We could go negative, but this isn't required by the spec, and negative numbers aren't
                // supported.
                self.total = self
                    .total
                    .checked_sub(self.deposits[did].amount)
                    .unwrap_or_default();
                self.frozen = true;
                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct ClientsDatabase {
    clients: HashMap<ClientId, Account>,
}

impl ClientsDatabase {
    pub fn process_transaction(
        &mut self,
        client_id: ClientId,
        t: Transaction,
    ) -> Result<(), crate::Error> {
        match self.clients.entry(client_id) {
            Entry::Occupied(mut occ) => occ.get_mut().process(t),
            Entry::Vacant(vac) => {
                if !matches!(t.kind, TransactionKind::Deposit) {
                    return Err(Error::AccountNotFound);
                }
                vac.insert(Default::default()).process(t)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (ClientId, &Account)> {
        self.clients.iter().map(|(k, v)| (*k, v))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        accounts::{Account, Transaction, TransactionKind::*},
        amount::Amount,
    };

    fn amount(v: &str) -> Amount {
        Amount::parse(v.as_bytes()).unwrap()
    }

    #[test]
    fn test_process_transaction_no_errors() {
        // Deposit 10.5
        let mut acc = Account::default();
        acc.process(Transaction {
            kind: Deposit,
            id: 0,
            amount: amount("10.5"),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("10.5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("10.5"));

        // Deposit 3. This will be disputed later.
        acc.process(Transaction {
            kind: Deposit,
            id: 1,
            amount: amount("3"),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("13.5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("13.5"));

        // Withdraw 2.
        acc.process(Transaction {
            kind: Withdrawal,
            id: 2,
            amount: amount("2"),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("11.5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("11.5"));

        // Dispute tx=1. This will end in resolution.
        acc.process(Transaction {
            kind: Dispute,
            id: 1,
            amount: Default::default(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("11.5"));
        assert_eq!(acc.held(), amount("3"));
        assert_eq!(acc.available_for_withdrawal(), amount("8.5"));

        // Resolve.
        acc.process(Transaction {
            kind: Resolve,
            id: 1,
            amount: Default::default(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("11.5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("11.5"));

        // Dispute tx=1 again. This will end in chargeback and account freeze.
        acc.process(Transaction {
            kind: Dispute,
            id: 1,
            amount: Default::default(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("11.5"));
        assert_eq!(acc.held(), amount("3"));
        assert_eq!(acc.available_for_withdrawal(), amount("8.5"));

        // Chargeback should freeze the account and make funds available for withdrawal 0.
        acc.process(Transaction {
            kind: Chargeback,
            id: 1,
            amount: Default::default(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("8.5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("0"));
        assert!(acc.frozen);
    }

    #[test]
    fn test_edge_case_chargeback_would_go_negative() {
        // Deposit 5
        let mut acc = Account::default();
        acc.process(Transaction {
            kind: Deposit,
            id: 0,
            amount: amount("5"),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("5"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("5"));

        // Withdraw 2
        acc.process(Transaction {
            kind: Withdrawal,
            id: 1,
            amount: amount("2"),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("3"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("3"));

        // Dispute the initial deposit. This will be resolved below.
        acc.process(Transaction {
            kind: Dispute,
            id: 0,
            amount: Amount::zero(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("3"));
        assert_eq!(acc.held(), amount("5"));
        assert_eq!(acc.available_for_withdrawal(), amount("0"));

        // Resolve it. It should release the funds.
        acc.process(Transaction {
            kind: Resolve,
            id: 0,
            amount: Amount::zero(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("3"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("3"));

        // Dispute again. Charging it back would make the account go negative.
        // Instead of going negative we set it to 0 and freeze to simplify the toy implementation.
        acc.process(Transaction {
            kind: Dispute,
            id: 0,
            amount: Amount::zero(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("3"));
        assert_eq!(acc.held(), amount("5"));
        assert_eq!(acc.available_for_withdrawal(), amount("0"));

        acc.process(Transaction {
            kind: Chargeback,
            id: 0,
            amount: Amount::zero(),
        })
        .unwrap();
        assert_eq!(acc.total(), amount("0"));
        assert_eq!(acc.held(), amount("0"));
        assert_eq!(acc.available_for_withdrawal(), amount("0"));
        assert!(acc.frozen);
    }

    #[test]
    fn test_withdraw_more_than_available() {
        let mut acc = Account::default();
        assert!(matches!(
            acc.process(Transaction {
                kind: Withdrawal,
                id: 0,
                amount: amount("1")
            })
            .unwrap_err(),
            Error::WithdrawOverflow
        ));

        acc.process(Transaction {
            kind: Deposit,
            id: 0,
            amount: amount("5"),
        })
        .unwrap();
        assert!(matches!(
            acc.process(Transaction {
                kind: Withdrawal,
                id: 1,
                amount: amount("6")
            })
            .unwrap_err(),
            Error::WithdrawOverflow
        ));

        assert!(
            acc.process(Transaction {
                kind: Withdrawal,
                id: 1,
                amount: amount("5")
            })
            .is_ok(),
        );
    }
}
