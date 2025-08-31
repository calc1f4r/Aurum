use anchor_lang::prelude::*;

#[error_code]
pub enum AurumError {
    #[msg("Config is already paused")]
    ConfigAlreadyPaused,
    #[msg("Config is already unpaused")]
    ConfigAlreadyUnpaused,
    #[msg("Tresury can't be same as the old one")]
    TresurySameAsOldOne,
    // MINT ERRORS 
    #[msg("Unsupport native mint token 2022")]
    UnsupportNativeMintToken2022,
    #[msg("Mint not supported")]
    MintNotSupported,
    #[msg("Invalid amount")] 
    InvalidAmount,
    #[msg("Math overflow")] 
    MathOverflow,
    #[msg("Unauthorized")] 
    Unauthorized,
    #[msg("Mint not whitelisted")] 
    MintNotWhitelisted,
    #[msg("Invalid market vault")] 
    InvalidVault,
    #[msg("Insufficient balance")] 
    InsufficientBalance,
    #[msg("Insufficient liquidity in market")] 
    InsufficientLiquidity,
    #[msg("Insufficient collateral for requested borrow")] 
    InsufficientCollateral,
    #[msg("Position not eligible for liquidation")]
    NotLiquidatable,
    #[msg("Invalid price feed data")]
    InvalidPriceFeed,
    #[msg("Invalid position")]
    InvalidPosition,
}