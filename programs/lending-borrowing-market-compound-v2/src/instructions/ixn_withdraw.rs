use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{USER_POSITION_SEED, MARKET_SEED};
use crate::errors::AurumError;
use crate::instructions::ixn_accrue_interest::accrue_market;
use crate::states::global_config::Config;
use crate::states::market::Market;
use crate::states::user_position::UserPosition;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, has_one = mint)]
    pub market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, Config>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint=market.mint,
        associated_token::authority=user.key(),
        constraint = user_destination.owner == user.key(),
    )]
    pub user_destination: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint=market.mint,
        associated_token::authority=user.key(),
        constraint = market_vault.key() == market.vault @ AurumError::InvalidVault,
    )]
    pub market_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED.as_bytes(), market.key().as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key(),
        constraint = user_position.market == market.key(),
    )]
    pub user_position: Account<'info, UserPosition>,

    pub token_program: Program<'info, Token>,
}

pub fn handler_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    // Accrue before any balance math
    accrue_market(&mut ctx.accounts.market, &ctx.accounts.config)?;

    let market = &mut ctx.accounts.market;
    let user_position = &mut ctx.accounts.user_position;

    // Compound user's supply principal up to current index
    if user_position.supply_principal > 0 {
        user_position.supply_principal = user_position.supply_principal * market.supply_index / user_position.supply_index.max(1);
    }
    user_position.supply_index = market.supply_index;

    // Ensure sufficient balance
    require!(user_position.supply_principal >= amount as u128, AurumError::InsufficientBalance);

    // Ensure market has enough cash
    require!(ctx.accounts.market_vault.amount >= amount, AurumError::InsufficientLiquidity);

    // Update state
    user_position.supply_principal = user_position.supply_principal - amount as u128;

    market.total_cash = market.total_cash - amount as u128;

    // Transfer tokens from market vault to user
    let market_key = market.key();
    require!(market_key == ctx.accounts.market_vault.owner, AurumError::InvalidVault);
    let market_seeds: &[&[u8]] = &[
        MARKET_SEED.as_bytes(),
        market.mint.as_ref(),
        &[market.bump],
    ];
    let signer: &[&[&[u8]]] = &[market_seeds];
    let cpi_accounts = Transfer {
        from: ctx.accounts.market_vault.to_account_info(),
        to: ctx.accounts.user_destination.to_account_info(),
        authority: market.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)?;

    Ok(())
}


