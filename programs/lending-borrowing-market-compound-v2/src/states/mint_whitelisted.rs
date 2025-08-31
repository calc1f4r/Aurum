use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct MintWhitelisted {
    pub mint: Pubkey,
    #[max_len(64)]
    pub pyth_price_feed: String,
    pub mint_decimals: u8,
    pub is_whitelisted: bool,
    pub bump: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintWhitelistedParams {
    pub pyth_price_feed: String,
    pub is_whitelisted: bool,
}
