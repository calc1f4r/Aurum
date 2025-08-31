use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{MARKET_SEED, USER_POSITION_SEED, BPS_DENOMINATOR},
    errors::AurumError,
    states::{global_config::Config, market::Market, user_position::UserPosition},
    utils::{compute_borrow_balance, compute_supply_balance},
};
use super::ixn_accrue_interest::accrue_market;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// CHECK : CHECKED Account
    pub user: AccountInfo<'info>,

    #[account(mut, has_one = mint)]
    pub asset_market: Account<'info, Market>,

    #[account(mut, has_one = mint)]
    pub credit_market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, Config>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = liquidator_source.mint == credit_market.mint,
        constraint = liquidator_source.owner == liquidator.key(),
    )]
    pub liquidator_source: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = liquidator_destination.mint == asset_market.mint,
        constraint = liquidator_destination.owner == liquidator.key(),
    )]
    pub liquidator_destination: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), credit_market.key().as_ref(), user.key().as_ref()],
        bump = credit_user_position.bump,
        constraint = credit_user_position.owner == user.key(),
        constraint = credit_user_position.market == credit_market.key(),
    )]
    pub credit_user_position: Account<'info, UserPosition>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), asset_market.key().as_ref(), user.key().as_ref()],
        bump = asset_user_position.bump,
        constraint = asset_user_position.owner == user.key(),
        constraint = asset_user_position.market == asset_market.key(),
    )]
    pub asset_user_position: Account<'info, UserPosition>,

    #[account(
        mut,
        constraint = credit_market_vault.mint == credit_market.mint,
        constraint = credit_market_vault.owner == credit_market.key(),
        constraint = credit_market_vault.key() == credit_market.vault @ AurumError::InvalidVault,
    )]
    pub credit_market_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = asset_market_vault.mint == asset_market.mint,
        constraint = asset_market_vault.owner == asset_market.key(),
        constraint = asset_market_vault.key() == asset_market.vault @ AurumError::InvalidVault,
    )]
    pub asset_market_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_liquidate(ctx: Context<Liquidate>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    let asset_market = &mut ctx.accounts.asset_market;
    let credit_market = &mut ctx.accounts.credit_market;
    let config = &ctx.accounts.config;
    let asset_user_position = &mut ctx.accounts.asset_user_position;
    let credit_user_position = &mut ctx.accounts.credit_user_position;

    // Accrue interest for both markets
    accrue_market(asset_market, config)?;
    accrue_market(credit_market, config)?;

    // Compound balances to current indices
    let (asset_supply_principal_now, asset_supply_balance) = compute_supply_balance(asset_user_position, asset_market)?;
    let (credit_borrow_principal_now, credit_borrow_balance) = compute_borrow_balance(credit_user_position, credit_market)?;
    asset_user_position.supply_principal = asset_supply_principal_now;
    asset_user_position.supply_index = asset_market.supply_index;
    credit_user_position.borrow_principal = credit_borrow_principal_now;
    credit_user_position.borrow_index = credit_market.borrow_index;

    // --- Solvency Check Placeholder ---
    // Without price or collateral factor per market we can't compute health factor precisely.
    // For now, enforce borrow > supply * liquidation_threshold (simple same-asset assumption).
    // If markets differ, this must be replaced with oracle-based USD valuation.
    let threshold_bps = config.liquidation_threshold as u128; // e.g., 8000 = 80%
    let max_borrow_allowed = asset_supply_balance * threshold_bps / BPS_DENOMINATOR;
    require!(credit_borrow_balance > max_borrow_allowed, AurumError::NotLiquidatable);

    // Determine repay (cannot exceed input or outstanding borrow)
    if credit_borrow_balance == 0 { return Ok(()); }
    let repay_amount_u128 = (amount as u128).min(credit_borrow_balance);
    let repay_amount_u64 = repay_amount_u128 as u64;

    // Seize amount = repay * (1 + bonus) where bonus is in bps
    let bonus_bps = config.liquidation_bonus as u128; // e.g., 500 = 5%
    let seize_amount = repay_amount_u128 * (BPS_DENOMINATOR + bonus_bps) / BPS_DENOMINATOR;

    require!(asset_supply_balance >= seize_amount, AurumError::InsufficientBalance);
    require!(asset_market.total_cash >= seize_amount, AurumError::InsufficientLiquidity);

    // Update credit side
    credit_user_position.borrow_principal = credit_user_position.borrow_principal - repay_amount_u128;
    credit_market.total_borrows = credit_market.total_borrows - repay_amount_u128;
    credit_market.total_cash = credit_market.total_cash + repay_amount_u128;

    // Update asset side (reduce user collateral, reduce market cash because transferring out)
    asset_user_position.supply_principal = asset_user_position.supply_principal - seize_amount;
    asset_market.total_cash = asset_market.total_cash - seize_amount;

    // Transfer debt repayment from liquidator -> credit market vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.liquidator_source.to_account_info(),
        to: ctx.accounts.credit_market_vault.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, repay_amount_u64)?;

    // Transfer seized collateral from asset market vault -> liquidator
    let asset_market_key = asset_market.key();
    require!(asset_market_key == ctx.accounts.asset_market_vault.owner, AurumError::InvalidVault);
    let asset_market_seeds: &[&[u8]] = &[
        MARKET_SEED.as_bytes(),
        asset_market.mint.as_ref(),
        &[asset_market.bump],
    ];
    let signer: &[&[&[u8]]] = &[asset_market_seeds];
    let cpi_accounts = Transfer {
        from: ctx.accounts.asset_market_vault.to_account_info(),
        to: ctx.accounts.liquidator_destination.to_account_info(),
        authority: asset_market.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, seize_amount as u64)?;

    Ok(())
}
