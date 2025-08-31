use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TransferChecked, TokenInterface};
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2};

use crate::{
    constants::{MARKET_SEED, USER_POSITION_SEED, BPS_DENOMINATOR},
    errors::AurumError,
    states::{global_config::GlobalConfig, market::Market, user_position::UserPosition},
    utils::{compute_borrow_balance, compute_supply_balance, get_asset_value_in_usd},
};
use super::ixn_accrue_interest::accrue_market;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// CHECK : CHECKED Account
    pub user: AccountInfo<'info>,

    #[account(mut)]
    pub asset_market: Account<'info, Market>,

    #[account(mut)]
    pub credit_market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, GlobalConfig>,

    #[account(constraint = mint.key() == credit_market.mint)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(constraint = asset_mint.key() == asset_market.mint)]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = liquidator_source.mint == credit_market.mint,
        constraint = liquidator_source.owner == liquidator.key(),
    )]
    pub liquidator_source: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = liquidator_destination.mint == asset_market.mint,
        constraint = liquidator_destination.owner == liquidator.key(),
    )]
    pub liquidator_destination: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), credit_market.key().as_ref(), user.key().as_ref()],
        bump = credit_user_position.bump,
        constraint = credit_user_position.owner == user.key(),
        constraint = credit_user_position.positions[0].market == credit_market.key(),
    )]
    pub credit_user_position: Account<'info, UserPosition>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), asset_market.key().as_ref(), user.key().as_ref()],
        bump = asset_user_position.bump,
        constraint = asset_user_position.owner == user.key(),
        constraint = asset_user_position.positions[0].market == asset_market.key(),
    )]
    pub asset_user_position: Account<'info, UserPosition>,

    #[account(
        mut,
        constraint = credit_market_vault.mint == credit_market.mint,
        constraint = credit_market_vault.owner == credit_market.key(),
        constraint = credit_market_vault.key() == credit_market.vault @ AurumError::InvalidVault,
    )]
    pub credit_market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = asset_market_vault.mint == asset_market.mint,
        constraint = asset_market_vault.owner == asset_market.key(),
        constraint = asset_market_vault.key() == asset_market.vault @ AurumError::InvalidVault,
    )]
    pub asset_market_vault: InterfaceAccount<'info, TokenAccount>,

    // Price feeds for asset and credit markets
    pub asset_price_update: Account<'info, PriceUpdateV2>,
    pub credit_price_update: Account<'info, PriceUpdateV2>,

    pub token_program: Interface<'info, TokenInterface>,
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
    asset_user_position.positions[0].supply_principal = asset_supply_principal_now;
    asset_user_position.positions[0].supply_index = asset_market.supply_index;
    credit_user_position.positions[0].borrow_principal = credit_borrow_principal_now;
    credit_user_position.positions[0].borrow_index = credit_market.borrow_index;

    // --- Health Factor Check with Price Feeds ---
    let maximum_age: u64 = 30; // 30 seconds max age for price feeds

    // Get asset (collateral) value in USD
    let asset_value_usd = get_asset_value_in_usd(
        asset_supply_balance,
        &ctx.accounts.asset_price_update,
        &asset_market.pyth_feed_id,
        maximum_age
    )?;

    // Get credit (borrow) balance value in USD
    let credit_value_usd = get_asset_value_in_usd(
        credit_borrow_balance,
        &ctx.accounts.credit_price_update,
        &credit_market.pyth_feed_id,
        maximum_age
    )?;

    // Check if position is undercollateralized (health factor < 1)
    // Health factor = collateral_value_usd / borrow_value_usd
    let threshold_bps = asset_market.liquidation_threshold_bps as u128; // e.g., 8000 = 80%
    let min_health_factor = threshold_bps * 100 / BPS_DENOMINATOR; // Convert to percentage (80%)
    let current_health_factor = if credit_value_usd > 0 {
        asset_value_usd * 100 / credit_value_usd
    } else {
        u128::MAX // No debt = infinite health
    };

    require!(current_health_factor < min_health_factor, AurumError::NotLiquidatable);

    // Determine repay (cannot exceed input or outstanding borrow)
    if credit_borrow_balance == 0 { return Ok(()); }
    let repay_amount_u128 = (amount as u128).min(credit_borrow_balance);
    let repay_amount_u64 = repay_amount_u128 as u64;

    // Seize amount = repay * (1 + bonus) where bonus is in bps
    let bonus_bps = asset_market.liquidation_bonus_bps as u128; // e.g., 500 = 5%
    let seize_amount = repay_amount_u128 * (BPS_DENOMINATOR + bonus_bps) / BPS_DENOMINATOR;

    require!(asset_supply_balance >= seize_amount, AurumError::InsufficientBalance);
    require!(asset_market.total_cash >= seize_amount, AurumError::InsufficientLiquidity);

    // Update credit side
    credit_user_position.positions[0].borrow_principal -= repay_amount_u128;
    credit_market.total_borrows -= repay_amount_u128;
    credit_market.total_cash += repay_amount_u128;

    // Update asset side (reduce user collateral, reduce market cash because transferring out)
    asset_user_position.positions[0].supply_principal -= seize_amount;
    asset_market.total_cash -= seize_amount;

    // Transfer debt repayment from liquidator -> credit market vault
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.liquidator_source.to_account_info(),
        to: ctx.accounts.credit_market_vault.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, repay_amount_u64, ctx.accounts.mint.decimals)?;

    // Transfer seized collateral from asset market vault -> liquidator
    let asset_market_key = asset_market.key();
    require!(asset_market_key == ctx.accounts.asset_market_vault.owner, AurumError::InvalidVault);
    let asset_market_seeds: &[&[u8]] = &[
        MARKET_SEED.as_bytes(),
        asset_market.mint.as_ref(),
        &[asset_market.bump],
    ];
    let signer: &[&[&[u8]]] = &[asset_market_seeds];
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.asset_market_vault.to_account_info(),
        to: ctx.accounts.liquidator_destination.to_account_info(),
        authority: asset_market.to_account_info(),
        mint: ctx.accounts.asset_mint.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token_interface::transfer_checked(cpi_ctx, seize_amount as u64, ctx.accounts.asset_mint.decimals)?;

    Ok(())
}
