#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("deposit overflowed - too much money in the account")]
    DepositOverflow,
    #[error("duplicate transaction id")]
    DuplicateTransactionId,
    #[error("withdraw overflowed - not enough money in the account")]
    WithdrawOverflow,
    #[error("transaction id not found")]
    TransactionNotFound,
    #[error("duplicate dispute")]
    DuplicateDispute,
    #[error("attempt to resolve undisputed transaction")]
    ResolveNotDisputed,
    #[error("attempt to chargeback undisputed transaction")]
    ChargebackNotDisputed,
    #[error("overflow increasing \"held\"")]
    HeldOverflow,
    #[error("account if frozen")]
    AccountFrozen,
    #[error("account not found")]
    AccountNotFound,

    #[error("CSV missing an expected column")]
    CsvMissingColumn,
    #[error("unknown transaction type")]
    CsvUnknownTransactionType,
    #[error("invalid client id")]
    CsvInvalidClientId,
    #[error("invalid transaction id")]
    CsvInvalidTxId,
    #[error("invalid amount")]
    CsvInvalidAmount,
    #[error("expected amount to be empty for this transaction type")]
    CsvUnexpectedAmount,
}
