use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use crate::states::mint_whitelisted::{MintWhitelisted, MintWhitelistedParams};
use crate::states::global_config::Config;
use crate::constants::MINT_WHITELISTED_SEED;

#[derive(Accounts)]
#[instruction(params: MintWhitelistedParams)]
pub struct WhitelistMint<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + MintWhitelisted::INIT_SPACE,
        seeds = [MINT_WHITELISTED_SEED.as_bytes(), mint.key().as_ref()],
        bump,
    )]
    pub mint_whitelisted: Account<'info, MintWhitelisted>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler_whitelist_mint(
    ctx: Context<WhitelistMint>,
    params: MintWhitelistedParams,
) -> Result<()> {
    let mint_whitelisted = &mut ctx.accounts.mint_whitelisted;

    let (_, bump) = Pubkey::find_program_address(&[MINT_WHITELISTED_SEED.as_bytes(), ctx.accounts.mint.key().as_ref()], ctx.program_id);

    if mint_whitelisted.bump != bump {
    // Update or initialize the mint whitelisted account
    mint_whitelisted.mint = ctx.accounts.mint.key();
    mint_whitelisted.bump = bump;
    mint_whitelisted.pyth_price_feed = params.pyth_price_feed;
    mint_whitelisted.mint_decimals = ctx.accounts.mint.decimals;
    mint_whitelisted.is_whitelisted = params.is_whitelisted;
    } 
    else{
        mint_whitelisted.is_whitelisted = params.is_whitelisted;
        mint_whitelisted.pyth_price_feed = params.pyth_price_feed;
    }
    
    Ok(())
}

