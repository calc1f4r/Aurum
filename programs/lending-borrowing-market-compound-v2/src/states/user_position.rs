use anchor_lang::prelude::*;

pub const MAX_POSITIONS: usize = 10;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct Position {
    pub market: Pubkey,
    pub supply_principal: u128,
    pub borrow_principal: u128,
    pub supply_index: u128,
    pub borrow_index: u128,
    pub is_collateral: u8, // 1 for true, 0 for false // 0 for a borrow position 
}

impl Position {
    pub fn is_active(&self) -> bool {
        self.market != Pubkey::default()
    }
}

#[derive(InitSpace)]
#[account]
pub struct UserPosition {
    pub bump: u8,
    pub owner: Pubkey,
    pub num_positions: u8,
    pub positions: [Position; MAX_POSITIONS],
}


