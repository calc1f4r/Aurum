use anchor_lang::prelude::*;

#[error_code]
pub enum AurumError {
    #[msg("Config is already paused")]
    ConfigAlreadyPaused,
    #[msg("Config is already unpaused")]
    ConfigAlreadyUnpaused,
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
    #[msg("Position not eligible for liquidation")]
    NotLiquidatable,
}