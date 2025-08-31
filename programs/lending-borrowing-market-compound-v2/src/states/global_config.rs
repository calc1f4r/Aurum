use anchor_lang::prelude::*;


#[derive(InitSpace)]
#[account]
pub struct Config {
    // This can withdraw the rates difference between borrow - supply (Protocol fees)
    pub treasury: Pubkey,
    pub admin: Pubkey,
    pub paused: bool,
    pub bump: u8,
}
