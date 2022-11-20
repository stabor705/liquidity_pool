use crate::error::{LiqPoolError, Result};

/// How 1 token is represented in u64 number.
/// Values less than UNIT are fractions. 1 is the smallest unit (ex. lamport in SOL).
pub const UNIT: u64 = 1000000000;

/// Calculate amount * (nominator / denominator)
pub fn propotion(amount: u64, nominator: u64, denominator: u64) -> Result<u64> {
    u64::try_from((amount as u128 * nominator as u128) / denominator as u128)
        .map_err(|_| LiqPoolError::CalculationError)
}

pub fn value(amount: u64, price: u64) -> Result<u64> {
    propotion(amount, price, UNIT)
}

/// Calculate someone's share after adding `value` to pool with `total_value`
/// of something and `total_share` of something
pub fn shares(value: u64, total_value: u64, total_shares: u64) -> Result<u64> {
    // first mint
    if total_shares == 0 {
        Ok(value)
    } else {
        propotion(value, total_shares, total_value)
    }
}

/// Given amount and a fee represented as a fraction in u64, calculate
/// amount with subtracted fee.
pub fn apply_fee(amount: u64, fee: u64) -> Result<u64> {
    Ok(amount - value(amount, fee)?)
}
