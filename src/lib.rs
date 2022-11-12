//! Implementation of mathematical model of unstake liquidity pool with
//! linear swap fee as described in [`Marinade documentation`].
//!
//! [`Marinade documentation`]: https://docs.marinade.finance/marinade-protocol/system-overview/unstake-liquidity-pool

mod calc;
pub mod error;
pub mod liq_pool;

pub use crate::liq_pool::LiqPool;
