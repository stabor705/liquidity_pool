use crate::calc::*;
use crate::error::{LiqPoolError, Result};

/// Mathematical model of unstake liquidity pool with linear swap fee.
pub struct LiqPool {
    max_fee: u64,
    min_fee: u64,
    liq_target: u64,

    token: u64,
    st_token: u64,
    lp_token_supply: u64,
}

impl LiqPool {

    pub fn new(max_fee: u64, min_fee: u64, liq_target: u64) -> LiqPool {
        if max_fee < min_fee {
            panic!("LiqPool: Max fee cannot be smaller than min fee");
        }
        LiqPool {
            max_fee,
            min_fee,
            liq_target,
            token: 0,
            st_token: 0,
            lp_token_supply: 0,
        }
    }

    /// Simulate putting tokens into liquidity pool.
    ///
    /// How much caller gets lp tokens in return
    /// depends on ratio between total liquidity pool value (token + st_token)
    /// and lp_token_supply.
    pub fn add_liquidity(&mut self, token_amount: u64) -> Result<u64> {
        let total_liq_pool_value = self.st_token + self.token;
        let lp_token_to_mint = shares(token_amount, total_liq_pool_value, self.lp_token_supply)?;
        self.token += token_amount;
        self.lp_token_supply += lp_token_to_mint;
        Ok(lp_token_to_mint)
    }

    /// Simulate removing liquidity from the pool.
    ///
    /// Caller gets token and st_token in propotion to their presence in liquidity pool.
    pub fn remove_liquidity(&mut self, lp_token_amount: u64) -> Result<(u64, u64)> {
        if lp_token_amount > self.lp_token_supply {
            return Err(LiqPoolError::InvalidInputData(
                "tried to remove more liquidity than it was possible with currently minted tokens"
                    .to_string(),
            ));
        }

        let token_amount = propotion(lp_token_amount, self.token, self.lp_token_supply)?;
        let st_token_amount = propotion(lp_token_amount, self.st_token, self.lp_token_supply)?;
        self.lp_token_supply -= lp_token_amount;
        self.token -= token_amount;
        self.st_token -= st_token_amount;
        Ok((token_amount, st_token_amount))
    }

    /// Simulate immediate unstake operation.
    ///
    /// User may request immediate unstake operation which allows getting
    /// tokens back, without delay, for a fee that depends lineary on current
    /// liquidity of the pool.
    pub fn swap(&mut self, st_token_amount: u64) -> Result<u64> {
        let fee = self.linear_fee(st_token_amount)?;
        let out_token_amount = apply_fee(st_token_amount, fee)?;
        if out_token_amount > self.token {
            return Err(LiqPoolError::InsufficientLiquidity);
        }

        self.token -= out_token_amount;
        self.st_token += st_token_amount;
        Ok(out_token_amount)
    }

    /// Compute fee based on st_token_amount swapped and current state of
    /// liquidity pool.
    fn linear_fee(&self, st_token_amount: u64) -> Result<u64> {
        if st_token_amount > self.token {
            return Ok(self.max_fee);
        }
        // Fee is computed based on liquidity AFTER swap operation.
        let liq_after = self.token - st_token_amount;
        if liq_after >= self.liq_target {
            Ok(self.min_fee)
        } else {
            Ok(self.max_fee - propotion(self.max_fee - self.min_fee, liq_after, self.liq_target)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_example_lp() -> LiqPool {
        LiqPool::new(3 * UNIT / 100, 3 * UNIT / 1000, 100000 * UNIT)
    }

    /* Simple testing single operations */

    // Adding liquidity should:
    // 1. return proper amount of lp token representing shares in the pool
    // 2. increase token amount in liq pool
    // 3. increase amount of minted lp tokens
    #[test]
    fn test_adding_liquidity() {
        let mut liq_pool = get_example_lp();
        let lp_token_amount = liq_pool.add_liquidity(500 * UNIT).unwrap();
        assert_eq!(lp_token_amount, 500 * UNIT);
        assert_eq!(liq_pool.lp_token_supply, 500 * UNIT);
        assert_eq!(liq_pool.token, 500 * UNIT)
    }

    // Removing liquidity should:
    // 1. return proper amount of token and st token
    // 2. decrease amount of token and st token in pool
    // 3. decrease amount of minted tokens
    #[test]
    fn test_removing_liquidity() {
        let mut liq_pool = get_example_lp();
        liq_pool.token = 500 * UNIT;
        liq_pool.st_token = 100 * UNIT;
        liq_pool.lp_token_supply = 600 * UNIT;

        let (token_amount, st_token_amount) = liq_pool.remove_liquidity(300 * UNIT).unwrap();
        assert_eq!(token_amount, 250 * UNIT);
        assert_eq!(st_token_amount, 50 * UNIT);
        assert_eq!(liq_pool.token, 250 * UNIT);
        assert_eq!(liq_pool.st_token, 50 * UNIT);

        let (token_amount, st_token_amount) = liq_pool.remove_liquidity(300 * UNIT).unwrap();
        assert_eq!(token_amount, 250 * UNIT);
        assert_eq!(st_token_amount, 50 * UNIT);
        assert_eq!(liq_pool.token, 0);
        assert_eq!(liq_pool.st_token, 0);
    }

    // Tests based on examples in marinade docs
    // https://docs.marinade.finance/marinade-protocol/system-overview/unstake-liquidity-pool

    #[test]
    fn test_linear_fee_with_target_reached() {
        let mut liq_pool = get_example_lp();
        liq_pool.add_liquidity(581250 * UNIT).unwrap();
        assert_eq!(liq_pool.linear_fee(90 * UNIT).unwrap(), 3 * UNIT / 1000);
    }

    #[test]
    fn test_linear_fee_with_target_not_reached() {
        let mut liq_pool = get_example_lp();
        liq_pool.add_liquidity(100030 * UNIT).unwrap();
        assert_eq!(liq_pool.linear_fee(9030 * UNIT).unwrap(), 543 * UNIT / 100000);
    }

    #[test]
    fn test_swapping_with_target_reached() {
        let mut liq_pool = get_example_lp();
        liq_pool.add_liquidity(581250 * UNIT).unwrap();
        assert_eq!(liq_pool.swap(90 * UNIT).unwrap(), 8973 * UNIT / 100)
    }

    #[test]
    fn test_swapping_with_target_not_reached() {
        let mut liq_pool = get_example_lp();
        liq_pool.add_liquidity(100030 * UNIT).unwrap();
        assert_eq!(liq_pool.swap(9030 * UNIT).unwrap(), 8980967100000);
    }

    /* Test error handling */

    #[test]
    fn test_removing_too_much_liquidity() {
        let mut liq_pool = get_example_lp();
        assert!(liq_pool.remove_liquidity(100).is_err());
    }

    #[test]
    fn test_unstaking_too_much() {
        let mut liq_pool = get_example_lp();
        assert!(liq_pool.swap(100).is_err());
    }

    /* Test complex scenerios */

    #[test]
    fn test_complex_scenerio() {
        let mut liq_pool = LiqPool::new(3 * UNIT / 100, 3 * UNIT / 1000, 500 * UNIT);
        // Alice puts 800 token in liq pool.
        liq_pool.add_liquidity(800 * UNIT).unwrap();
        // Bob could not wait and used immediate unstake with 300 st token.
        // He pays 0.3% fee because target liquidity is reached even after
        // his unstake. That means he gets 300 - 0.3% = 299.1 token amount.
        let token_amount = liq_pool.swap(300 * UNIT).unwrap();
        assert_eq!(token_amount, 2991 * UNIT / 10);
        assert_eq!(liq_pool.token, 5009 * UNIT / 10);
        assert_eq!(liq_pool.st_token, 300 * UNIT);
        assert_eq!(liq_pool.lp_token_supply, 800 * UNIT);
        // Now Carlos also unstakes 300. This time he pays bigger fee, because
        // after his unstake liquidity target will not be reached.
        // He pays 3% - ((3% - 0.3%) * (liq_after/liq_target))
        //       = 3% - 2.7% * (500.9 - 300)/500
        //       = 1.91514%
        // 300 - 1.91514% = 294.25458
        let token_amount = liq_pool.swap(300 * UNIT).unwrap();
        assert_eq!(token_amount, 29425458 * UNIT / 100000);
        assert_eq!(liq_pool.token, 20664542 * UNIT / 100000);
        assert_eq!(liq_pool.st_token, 600 * UNIT);
        assert_eq!(liq_pool.lp_token_supply, 800 * UNIT);
        // David adds liquidity to the pool. Because this time total value is
        // a little bit more than total shares (806.64542 > 800) he should get
        // less lp tokens than his friends.
        // His share is 400 * (800 / 806.64542)
        let lp_token_amount = liq_pool.add_liquidity(400 * UNIT).unwrap();
        assert_eq!(lp_token_amount, 396704663617);
        assert_eq!(liq_pool.token, 60664542 * UNIT / 100000);
        assert_eq!(liq_pool.st_token, 600 * UNIT);
        assert_eq!(liq_pool.lp_token_supply, 1196704663617);
        // Finally, let's say Alice removes liquidity for fun and profits.
        // She should earn more token amount now because of the fees that
        // have been collected.
        // token_amount = 200 * (token in liq pool) / (lp token supply) = 101.385987444
        // st_token_amount = 200 * (st toke in liq pool) / (lp token supply) = 100.275367555
        let (token_amount, st_token_amount) = liq_pool.remove_liquidity(200 * UNIT).unwrap();
        assert_eq!(token_amount, 101385987444);
        assert_eq!(st_token_amount, 100275367555);
        assert_eq!(liq_pool.token, 505259432556);
        assert_eq!(liq_pool.st_token, 499724632445);
        assert_eq!(liq_pool.lp_token_supply, 996704663617);
    }
}
