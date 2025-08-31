use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct UserPosition {
    pub bump: u8,
    pub market: Pubkey,
    pub owner: Pubkey,

    // Principal amounts
    pub supply_principal: u128,
    pub borrow_principal: u128,

    // Indexes captured at last update (mantissa scaled)
    pub supply_index: u128,
    pub borrow_index: u128,
}


