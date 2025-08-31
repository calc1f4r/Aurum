use anchor_lang::prelude::*;


#[derive(InitSpace)]
#[account]
pub struct Config {
    // --- Protocol Admin & State ---
    pub bump: u8,
    pub admin: Pubkey,
    pub paused: bool,
    pub updated_at: u64,

    // --- Liquidation Params (Often Market-Specific) ---
    /// The percentage at which a loan is considered undercollateralized and can be liquidated.
    /// Example: 8000 for 80%.
    pub liquidation_threshold: u64,
    
    /// The bonus percentage a liquidator receives on the repaid value.
    /// Example: 500 for a 5% bonus.
    pub liquidation_bonus: u64,

    /// The base borrow rate when utilization is 0.
    /// Stored as a scaled integer (e.g., with 1e18 precision).
    pub base_rate: u64,
    
    /// The multiplier for the interest rate slope before the kink point.
    /// Stored as a scaled integer.
    pub slope1: u64,
    
    /// The multiplier for the interest rate slope after the kink point.
    /// Stored as a scaled integer.
    pub jump_slope: u64,
    
    /// The utilization rate at which the interest rate slope switches from `slope1` to `jump_slope`.
    /// Stored as a scaled integer. Example: 8000 for 80% utilization.
    pub kink: u64,
    
    /// The fraction of interest paid by borrowers that is diverted to protocol reserves.
    /// Stored as a scaled integer. Example: 1000 for 10%.
    pub reserve_factor: u64,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct GlobalConfigParams {
    pub base_rate: u64,
    pub slope1: u64,
    pub jump_slope: u64,
    pub kink: u64,
    pub reserve_factor: u64,
    pub liquidation_threshold: u64,
    pub liquidation_bonus: u64,
}   