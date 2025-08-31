use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Market {
    pub bump: u8,
    pub mint: Pubkey,
    pub vault: Pubkey,

    // Totals in base token units unless stated otherwise
    pub total_cash: u128,
    pub total_borrows: u128,
    pub total_reserves: u128,

    // cToken for collateral
    pub c_token_mint: Pubkey,
    pub c_token_supply: u128,

    pub pyth_feed_id: [u8; 32], // Pyth price feed ID for this asset
    pub decimals: u8, // Decimals of the mint

    // Interest indices scaled by mantissa (e.g., 1e18)
    pub borrow_index: u128,
    pub supply_index: u128,
    // Risk configuration (per-market)
    pub ltv_bps: u16,                    // Max borrow allowed vs collateral value
    pub liquidation_threshold_bps: u16,   // Health threshold for liquidation
    pub liquidation_bonus_bps: u16,       // Seizer bonus
    pub reserve_factor_bps: u16,          // Interest to reserves (protocol share)
    // Interest rate model
    pub kink_utilization_bps: u16,        // Utilization kink
    pub base_rate_per_year: u64,          // All annual rates are mantissa 1e18
    pub slope1_per_year: u64,
    pub slope2_per_year: u64,
    pub last_updated_ts: u64,

}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MarketInitParams {
    pub pyth_feed_id: [u8; 32],
    pub decimals: u8,
    pub ltv_bps: u16,
    pub liquidation_threshold_bps: u16,
    pub liquidation_bonus_bps: u16,
    pub reserve_factor_bps: u16,
    pub kink_utilization_bps: u16,
    pub base_rate_per_year: u64,
    pub slope1_per_year: u64,
    pub slope2_per_year: u64,
}

impl Default for MarketInitParams {
    fn default() -> Self {
        Self {
            pyth_feed_id: [0; 32], // Default empty feed ID
            decimals: 9, // Default to 9 decimals like SOL
            ltv_bps: 7500,
            liquidation_threshold_bps: 8000,
            liquidation_bonus_bps: 500,
            reserve_factor_bps: 1000,
            kink_utilization_bps: 8000,
            base_rate_per_year: 0,
            slope1_per_year: 50_000_000_000_000_000, // 5% in 1e18
            slope2_per_year: 200_000_000_000_000_000, // 20% in 1e18
        }
    }
}


