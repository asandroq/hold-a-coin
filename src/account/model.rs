#![warn(missing_docs)]

/*!
 * Module that implements the client accounts domain.
 *
 * As a domain model, this module does not concern itself with I/O or
 * data serialisation.
 */

use std::error;
use std::num::FpCategory;
use std::convert::TryFrom;
use std::fmt;

/// Domain errors.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// Errors such as arithmetic overflow or underflow.
    Arithmetic,

    /// Not enough funds for processing transactions.
    InsufficientFunds,
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Arithmetic => write!(fmt, "Arithmetic error"),
            Error::InsufficientFunds => write!(fmt, "Funds Insufficient for operation"),
        }
    }
}

impl error::Error for Error {}

/// Alias for results that use our domain errors.
pub type Result<T> = std::result::Result<T, Error>;

/// Thin wrapper around monetary amounts.
///
/// There are many possibilities for modeling monetary amounts within
/// a computer. Using floating-point numbers would be the simplest
/// choice, but they cannot represent exactly all decimal fractions
/// with four digits of precision, which can be demonstrated with the
/// classic [example of `0.1 + 0.2`](https://0.30000000000000004.com/).
///
/// Therefore, we model monetary amounts with unsigned 64-bit
/// numbers. We need 64 bits because here at Hold-a-Coin we cater
/// primarily to the billionaire market segment. The type is wrapped
/// within a struct in order to forbid the use of regular arithmetic
/// operators on it.
///
/// This type holds the quantity of 1/10000 fractions of a coin.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Amount(u64);

impl Amount {

    fn new() -> Self {
        Amount(0)
    }

    /// Adds another Amount to this one,  while detecting overflows.
    pub fn add(self, other: Self) -> Result<Amount> {
        let (val, over) = self.0.overflowing_add(other.0);
        if over {
            Err(Error::Arithmetic)
        } else {
            Ok(Amount(val))
        }
    }

    /// Subtracts another Amount from this one, while detecting underflows.
    pub fn sub(self, other: Self) -> Result<Amount> {
        let (val, under) = self.0.overflowing_sub(other.0);
        if under {
            Err(Error::Arithmetic)
        } else {
            Ok(Amount(val))
        }
    }
}

/// Display the amount as a floating-point number for user interfaces.
impl fmt::Display for Amount {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let floating = self.0 as f64 / 10000.0;
        return write!(fmt, "{:.4}", floating)
    }
}

impl TryFrom<f64> for Amount {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        let tmp  = value * 10000.0;
        let class = tmp.classify();
        if class == FpCategory::Zero || class == FpCategory::Normal && tmp > 0.0 {
            Ok(Amount(tmp as u64))
        } else {
            Err(Error::Arithmetic)
        }
    }
}

/// Alias for a client identifier.
///
/// It's wrapped in a struct in order to forbid arithmetic and
/// ordering on it.
 #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(u16);

impl fmt::Display for ClientId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

impl From<u16> for ClientId {
    fn from(id: u16) -> ClientId {
        ClientId(id)
    }
}

/// Alias for a transaction identifier.
///
/// It's wrapped in a struct in order to forbid arithmetic and
/// ordering on it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Tx(u32);

impl From<u32> for Tx {
    fn from(tx: u32) -> Tx {
        Tx(tx)
    }
}

/// The extant types of transactions.
#[derive(Debug, Eq, PartialEq)]
pub enum Transaction {
    /// A credit to the client's asset account.
    ///
    /// It has an identifier and the amount that was deposited.
    Deposit(Tx, Amount),

    /// A debit to the client's asset account.
    ///
    /// It has an identifier and the amount that was withdrawn.
    Withdrawal(Tx, Amount),

    /// A client's claim that a transaction was erroneous.
    ///
    /// It has the identifier of the transaction that's being
    /// disputed.
    Dispute(Tx),

    /// A resolution to a dispute.
    ///
    /// It has the identifier of the transaction that's being
    /// resolved.
    Resolve(Tx),

    /// The final state of a dispute.
    ///
    /// It has the identifier of the transaction that will be
    /// reversed.
    Chargeback(Tx),
}

/// A single transaction against a client account.
#[derive(Debug, Eq, PartialEq)]
pub struct ClientTransaction {
    /// The account this transaction is related to.
    client_id: ClientId,

    /// The contents of this transaction.
    transaction: Transaction,
}

/// A deposit stored in the client's account.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Deposit {
    /// The identifier of the transaction for this deposit.
    tx: Tx,

    /// The amount that was deposited.
    amount: Amount,

    /// If this deposit is currently being disputed.
    disputed: bool,
}

impl Deposit {
    fn new(tx: Tx, amount: Amount) -> Self {
        Deposit { tx, amount, disputed: false }
    }
}

/// A client's account.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Account {
    /// The owner of this account.
    pub owner: ClientId,

    /// The available funds.
    pub available: Amount,

    /// Funds held in disputes.
    pub held: Amount,

    /// If the account is locked due to a chargeback.
    pub locked: bool,

    /// Log of deposits
    deposits: Vec<Deposit>,
}

impl Account {
    pub fn new(client_id: ClientId) -> Self {
        Account {
            owner: client_id,
            available: Amount::new(),
            held: Amount::new(),
            locked: false,
            deposits: vec![],
        }
    }

    pub fn apply(&mut self, tx: Transaction) -> Result<()> {
        match tx {
            Transaction::Deposit(id, amount) => {
                let new_avail = self.available.add(amount)?;
                self.available = new_avail;
                self.deposits.push(Deposit::new(id, amount));
                Ok(())
            },
           Transaction::Withdrawal(_, amount) => {
                let new_avail = self.available.sub(amount).map_err(|_| Error::InsufficientFunds)?;
                self.available = new_avail;
                Ok(())
            },
            Transaction::Dispute(other_tx) => {
                let maybe_dep = self.deposits.iter_mut().find(|d| d.tx == other_tx);
                match maybe_dep {
                    Some(dep) if !dep.disputed => {
                        let new_avail = self.available.sub(dep.amount).map_err(|_| Error::InsufficientFunds)?;
                        let new_held = self.held.add(dep.amount)?;
                        self.available = new_avail;
                        self.held = new_held;
                        dep.disputed = true;
                        Ok(())
                    },
                    _ => Ok(())
                }
            },
            Transaction::Resolve(other_tx) => {
                let maybe_dep = self.deposits.iter_mut().find(|d| d.tx == other_tx);
                match maybe_dep {
                    Some(dep) if dep.disputed => {
                        let new_held = self.held.sub(dep.amount).map_err(|_| Error::InsufficientFunds)?;
                        let new_avail = self.available.add(dep.amount)?;
                        self.available = new_avail;
                        self.held = new_held;
                        dep.disputed = false;
                        Ok(())
                    },
                    _ => Ok(())
                }
            },
            Transaction::Chargeback(other_tx) => {
                let maybe_dep = self.deposits.iter().find(|d| d.tx == other_tx);
                match maybe_dep {
                    Some(dep) if dep.disputed => {
                        let new_held = self.held.sub(dep.amount).map_err(|_| Error::InsufficientFunds)?;
                        self.held = new_held;
                        self.locked = true;
                        Ok(())
                    },
                    _ => Ok(())
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use ::quickcheck::TestResult;
    use ::quickcheck_macros::quickcheck;
    use super::*;

    #[test]
    fn test_amount_from() {
        let twelve_3 = Amount::try_from(12.3).unwrap();
        assert_eq!(twelve_3.0, 123000);

        assert!(Amount::try_from((-1.0f64).sqrt()).is_err());
        assert!(Amount::try_from(f64::MAX).is_err());
    }

    #[test]
    fn test_amount_arithmetic() {
        assert_eq!(
            Amount::try_from(99.34).and_then(|a|
                Amount::try_from(53.44).and_then(|b|
                    a.add(b)
                )
            ),
            Amount::try_from(152.78),
        );

        assert_eq!(
            Amount::try_from(41.1).and_then(|a|
                Amount::try_from(11.9).and_then(|b|
                    a.sub(b)
                )
            ),
            Amount::try_from(29.2),
        );

        assert!(
            Amount::try_from(u64::MAX as f64).and_then(|a|
                Amount::try_from(41.1).and_then(|b|
                    a.add(b)
                )
            ).is_err()
        );

        assert!(
            Amount::try_from(11.9).and_then(|a|
                Amount::try_from(41.1).and_then(|b|
                    a.sub(b)
                )
            ).is_err()
        );
    }

    #[quickcheck]
    fn amount_add_sub(value: f64) -> TestResult {
        if value.is_sign_negative() || value.is_nan() || value.is_infinite() || value > f64::MAX / 10000.0 {
            TestResult::discard()
        } else {
            let zero = Amount::new();
            let amt = Amount::try_from(value).unwrap();
            let added = zero.add(amt).unwrap();
            let subbed = added.sub(amt).unwrap();
            TestResult::from_bool(subbed == zero)
        }
    }
}
