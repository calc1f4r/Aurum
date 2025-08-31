use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TransferChecked, TokenInterface};

use crate::{
    constants::USER_POSITION_SEED,
    errors::AurumError,
    states::{global_config::GlobalConfig, market::Market, user_position::UserPosition},
    utils::{compute_borrow_balance},
};
use super::ixn_accrue_interest::accrue_market;

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, has_one = mint)]
    pub market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        // User's associated token account for the repay mint
        associated_token::mint = mint.key(),
        associated_token::authority = user.key(),
    )]
    pub user_source: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = market.mint,
        associated_token::authority = market,
        constraint = market_vault.key() == market.vault @ AurumError::InvalidVault,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), market.key().as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
        constraint = user_position.positions[0].market == market.key(),
    )]
    pub user_position: Account<'info, UserPosition>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler_repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    let market = &mut ctx.accounts.market;
    let config = &ctx.accounts.config;
    let user_position = &mut ctx.accounts.user_position;

    // Accrue interest before any balance math
    accrue_market(market, config)?;

    // Compound borrow to current index
    let (current_borrow_principal, current_borrow_balance) = compute_borrow_balance(user_position, market)?;
    user_position.positions[0].borrow_principal = current_borrow_principal; // align principal with compounded amount
    user_position.positions[0].borrow_index = market.borrow_index;

    if current_borrow_balance == 0 { return Ok(()); }

    let repay_amount_u128 = (amount as u128).min(current_borrow_balance);
    let repay_amount_u64 = repay_amount_u128 as u64; // safe because repay input was u64 and min logic

    // Update state (principal decreases directly)
    user_position.positions[0].borrow_principal -= repay_amount_u128;

    market.total_borrows -= repay_amount_u128;

    market.total_cash += repay_amount_u128;

    // Transfer tokens from user to market vault
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.user_source.to_account_info(),
        to: ctx.accounts.market_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, repay_amount_u64, ctx.accounts.mint.decimals)?;

    Ok(())
}
