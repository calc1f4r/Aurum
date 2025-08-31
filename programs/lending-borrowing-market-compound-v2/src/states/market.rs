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

    // Interest indices scaled by mantissa (e.g., 1e18)
    pub borrow_index: u128,
    pub supply_index: u128,


    
    pub last_updated_ts: u64,

}


