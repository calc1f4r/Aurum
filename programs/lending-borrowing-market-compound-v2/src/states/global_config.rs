use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GlobalConfigParams {
    pub liquidation_threshold: u16, // in basis points (e.g., 8000 = 80%)
    pub liquidation_bonus: u16,     // in basis points (e.g., 500 = 5%)
    pub base_rate: u16,             // in basis points
    pub slope1: u16,                // in basis points
    pub jump_slope: u16,            // in basis points
    pub kink: u16,                  // in basis points (utilization ratio where rate model changes)
    pub reserve_factor: u16,        // in basis points (protocol fee)
}

#[derive(InitSpace)]
#[account]
pub struct Config {
    // This can withdraw the rates difference between borrow - supply (Protocol fees)
    pub treasury: Pubkey,
    pub admin: Pubkey,
    pub paused: bool,
    pub bump: u8,
    
    // Interest rate model parameters
    pub liquidation_threshold: u16, // in basis points (e.g., 8000 = 80%)
    pub liquidation_bonus: u16,     // in basis points (e.g., 500 = 5%)
    pub base_rate: u16,             // in basis points
    pub slope1: u16,                // in basis points
    pub jump_slope: u16,            // in basis points
    pub kink: u16,                  // in basis points (utilization ratio where rate model changes)
    pub reserve_factor: u16,        // in basis points (protocol fee)
    pub updated_at: u64,            // timestamp of last update
}
