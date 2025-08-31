use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{USER_POSITION_SEED, MARKET_SEED};
use crate::states::market::Market;
use crate::states::user_position::UserPosition;
use crate::states::global_config::Config;
use crate::states::mint_whitelisted::MintWhitelisted;
use crate::constants::MINT_WHITELISTED_SEED;

use crate::errors::AurumError;
use crate::instructions::ixn_accrue_interest::accrue_market;
use crate::utils::compute_supply_balance;

#[derive(Accounts)]
pub struct Borrow<'info> {
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
        associated_token::mint = mint.key(),
        associated_token::authority = user.key(),
    )]
    pub user_destination: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint.key(),
        associated_token::authority = market.key(),
    )]
    pub market_vault: Account<'info, TokenAccount>,

    #[account(
        seeds = [MINT_WHITELISTED_SEED.as_bytes(), mint.key().as_ref()],
        bump = mint_whitelisted.bump,
        constraint = mint_whitelisted.is_whitelisted @ AurumError::MintNotWhitelisted,
    )]
    pub mint_whitelisted: Account<'info, MintWhitelisted>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserPosition::INIT_SPACE,
        seeds = [USER_POSITION_SEED.as_bytes(), market.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_position: Account<'info, UserPosition>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler_borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    // Accrue before updating indices and balances
    accrue_market(&mut ctx.accounts.market, &ctx.accounts.config)?;

    let market = &mut ctx.accounts.market;
    let user_position = &mut ctx.accounts.user_position;

    // Check if market has enough liquidity
    require!(market.total_cash >= amount as u128, AurumError::InsufficientLiquidity);

    // Initialize user position if needed
    if user_position.market == Pubkey::default() {
        let (_pda, bump) = Pubkey::find_program_address(
            &[USER_POSITION_SEED.as_bytes(), market.key().as_ref(), ctx.accounts.user.key().as_ref()],
            ctx.program_id,
        );
        user_position.bump = bump;
        user_position.market = market.key();
        user_position.owner = ctx.accounts.user.key();
        user_position.supply_index = market.supply_index;
        user_position.borrow_index = market.borrow_index;
        user_position.supply_principal = 0;
        user_position.borrow_principal = 0;
    }

    // Compound the existing borrow principal to current index before adding
    if user_position.borrow_principal > 0 {
        user_position.borrow_principal = user_position.borrow_principal * market.borrow_index / user_position.borrow_index.max(1);
    }
    user_position.borrow_index = market.borrow_index;

    // Basic solvency check - user must have some collateral
    // For a more sophisticated check, this would compare borrow value to collateral value using oracles
    if user_position.supply_principal == 0 {
        return Err(AurumError::InsufficientBalance.into());
    }

    // Simple collateralization check (assumes same asset for now)
    let (_, current_supply_balance) = compute_supply_balance(user_position, market)?;
    let new_borrow_balance = user_position.borrow_principal + amount as u128;
    let max_borrow_allowed = current_supply_balance * ctx.accounts.config.liquidation_threshold as u128 / 10000;
    
    require!(new_borrow_balance <= max_borrow_allowed, AurumError::InsufficientBalance);

    // Update user position
    user_position.borrow_principal = user_position.borrow_principal + amount as u128;

    // Update market totals
    market.total_cash = market.total_cash - amount as u128;
    market.total_borrows = market.total_borrows + amount as u128;

    // Transfer tokens from market vault to user
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