use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;
use crate::constants::{MARKET_SEED, INDEX_MANTISSA, MINT_WHITELISTED_SEED};
use crate::states::market::{Market, MarketInitParams};
use crate::states::global_config::GlobalConfig;
use crate::states::mint_whitelisted::MintWhitelisted;
use crate::errors::AurumError;

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        seeds = [MINT_WHITELISTED_SEED.as_bytes(), mint.key().as_ref()],
        bump = mint_whitelisted.bump,
        constraint = mint_whitelisted.is_whitelisted @ AurumError::MintNotWhitelisted,
    )]
    pub mint_whitelisted: Account<'info, MintWhitelisted>,

    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [MARKET_SEED.as_bytes(), mint.key().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = admin,
        associated_token::authority = market,
        associated_token::mint = mint,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler_initialize_market(ctx: Context<InitializeMarket>, params: MarketInitParams) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.pyth_feed_id = params.pyth_feed_id;
    market.mint = ctx.accounts.mint.key();
    market.vault = ctx.accounts.vault.key();
    market.total_cash = 0;
    market.total_borrows = 0;
    market.total_reserves = 0;
    market.borrow_index = INDEX_MANTISSA;
    market.supply_index = INDEX_MANTISSA; // initial exchange rate
    market.last_updated_ts = Clock::get()?.unix_timestamp as u64;
    // Populate per-market params
    market.ltv_bps = params.ltv_bps;
    market.liquidation_threshold_bps = params.liquidation_threshold_bps;
    market.liquidation_bonus_bps = params.liquidation_bonus_bps;

    Ok(())
}


