use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiqPoolError {
    #[error("Program tried to do erroneous calculation")]
    CalculationError,
    #[error("A logically impossible input value: {0}")]
    InvalidInputData(String),
    #[error("Liquidity of the pool was to small to execute operation")]
    InsufficientLiquidity,
}

pub type Result<T> = std::result::Result<T, LiqPoolError>;
