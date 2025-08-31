use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TransferChecked, TokenInterface};
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2};

use crate::constants::{USER_POSITION_SEED, MARKET_SEED, BPS_DENOMINATOR};
use crate::errors::AurumError;
use crate::states::{global_config::GlobalConfig, market::Market, user_position::UserPosition};
use crate::utils::{compute_borrow_balance, compute_supply_balance, get_asset_value_in_usd};
use super::ixn_accrue_interest::accrue_market;

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, constraint = collateral_market.mint == collateral_mint.key())]
    pub collateral_market: Account<'info, Market>, // market where user has supplied collateral

    #[account(mut, constraint = borrow_market.mint == borrow_mint.key())]
    pub borrow_market: Account<'info, Market>, // market the user wants to borrow from

    #[account(constraint = !config.paused)]
    pub config: Account<'info, GlobalConfig>,

    /// CHECK: Mint keys verified via markets
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Mint keys verified via markets
    pub borrow_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = borrow_mint,
        associated_token::authority = user.key(),
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = borrow_market_vault.mint == borrow_mint.key(),
        constraint = borrow_market_vault.owner == borrow_market.key(),
        constraint = borrow_market_vault.key() == borrow_market.vault @ AurumError::InvalidVault,
    )]
    pub borrow_market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), collateral_market.key().as_ref(), user.key().as_ref()],
        bump = collateral_user_position.bump,
        constraint = collateral_user_position.owner == user.key(),
        constraint = collateral_user_position.positions[0].market == collateral_market.key(),
    )]
    pub collateral_user_position: Account<'info, UserPosition>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [USER_POSITION_SEED.as_bytes(), borrow_market.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub borrow_user_position: Account<'info, UserPosition>,

    // Price feeds for collateral and borrow assets
    pub collateral_price_update: Account<'info, PriceUpdateV2>,
    pub borrow_price_update: Account<'info, PriceUpdateV2>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler_borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    let collateral_market = &mut ctx.accounts.collateral_market;
    let borrow_market = &mut ctx.accounts.borrow_market;
    let config = &ctx.accounts.config; // for pause only presently
    let collateral_user_position = &mut ctx.accounts.collateral_user_position;
    let borrow_user_position = &mut ctx.accounts.borrow_user_position;

    // Accrue interest on both markets
    accrue_market(collateral_market, config)?;
    accrue_market(borrow_market, config)?;

    // Update collateral position supply principal to current index
    let (collateral_supply_principal_now, collateral_supply_balance) = compute_supply_balance(collateral_user_position, collateral_market)?;
    collateral_user_position.positions[0].supply_principal = collateral_supply_principal_now;
    collateral_user_position.positions[0].supply_index = collateral_market.supply_index;

    // Initialize borrow position if new
    if borrow_user_position.positions[0].market == Pubkey::default() {

        borrow_user_position.bump = ctx.bumps.borrow_user_position;
        borrow_user_position.positions[0].market = borrow_market.key();
        borrow_user_position.owner = ctx.accounts.user.key();
        borrow_user_position.positions[0].supply_index = borrow_market.supply_index;
        borrow_user_position.positions[0].borrow_index = borrow_market.borrow_index;
        borrow_user_position.positions[0].supply_principal = 0;
        borrow_user_position.positions[0].borrow_principal = 0;
        borrow_user_position.positions[0].is_collateral = 0;
    }

    // Compound existing borrow to current index
    let (borrow_principal_now, borrow_balance_now) = compute_borrow_balance(borrow_user_position, borrow_market)?;
    borrow_user_position.positions[0].borrow_principal = borrow_principal_now;
    borrow_user_position.positions[0].borrow_index = borrow_market.borrow_index;

    // Get price feeds for proper collateral valuation
    let maximum_age: u64 = 30; // 30 seconds max age for price feeds

    // Get collateral value in USD
    let collateral_value_usd = get_asset_value_in_usd(
        collateral_supply_balance,
        &ctx.accounts.collateral_price_update,
        &collateral_market.pyth_feed_id,
        maximum_age
    )?;

    // Get existing borrow balance value in USD
    let borrow_balance_now_usd = get_asset_value_in_usd(
        borrow_balance_now,
        &ctx.accounts.borrow_price_update,
        &borrow_market.pyth_feed_id,
        maximum_age
    )?;

    // Get borrow amount value in USD
    let borrow_amount_usd = get_asset_value_in_usd(
        amount as u128,
        &ctx.accounts.borrow_price_update,
        &borrow_market.pyth_feed_id,
        maximum_age
    )?;

    let new_borrow_balance_usd = borrow_balance_now_usd + borrow_amount_usd;

    // Check LTV: new_borrow_balance_usd <= collateral_value_usd * ltv
    let max_borrow_allowed_usd = collateral_value_usd * (collateral_market.ltv_bps as u128) / BPS_DENOMINATOR;
    require!(new_borrow_balance_usd <= max_borrow_allowed_usd, AurumError::InsufficientCollateral);

    // Market liquidity
    require!(borrow_market.total_cash >= amount as u128, AurumError::InsufficientLiquidity);

    // State updates
    let new_borrow_balance = borrow_balance_now + amount as u128; // Keep in token units for accounting
    borrow_user_position.positions[0].borrow_principal = new_borrow_balance; // principal becomes full compounded + new amount
    borrow_market.total_borrows += amount as u128;
    borrow_market.total_cash -= amount as u128;

    // Transfer tokens from borrow market vault to user
    let market_seeds: &[&[u8]] = &[
        MARKET_SEED.as_bytes(),
        borrow_market.mint.as_ref(),
        &[borrow_market.bump],
    ];
    let signer: &[&[&[u8]]] = &[market_seeds];
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.borrow_market_vault.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: borrow_market.to_account_info(),
        mint: ctx.accounts.borrow_mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    token_interface::transfer_checked(cpi_ctx, amount, ctx.accounts.borrow_mint.decimals)?;

    Ok(())
}
