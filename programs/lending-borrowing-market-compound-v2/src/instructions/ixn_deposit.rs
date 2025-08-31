use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{USER_POSITION_SEED};
use crate::states::market::Market;
use crate::states::user_position::UserPosition;
use crate::states::global_config::GlobalConfig;
use crate::states::mint_whitelisted::MintWhitelisted;
use crate::constants::MINT_WHITELISTED_SEED;

use crate::errors::AurumError;
use crate::instructions::ixn_accrue_interest::accrue_market;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, has_one = mint)]
    pub market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint.key(),
        associated_token::authority = user.key(),
    )]
    pub user_source: Account<'info, TokenAccount>,

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

pub fn handler_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, AurumError::InvalidAmount);

    // Accrue before updating indices and balances
    accrue_market(&mut ctx.accounts.market, &ctx.accounts.config)?;

    // Transfer tokens from user to market vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_source.to_account_info(),
        to: ctx.accounts.market_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update totals and user principal
    let market = &mut ctx.accounts.market;
    // Overflow will panic (overflow-checks enabled in Cargo profile)
    market.total_cash += amount as u128;  

    let user_position = &mut ctx.accounts.user_position;
    if user_position.positions[0].market == Pubkey::default() {
        // init
        let (_pda, bump) = Pubkey::find_program_address(
            &[USER_POSITION_SEED.as_bytes(), market.key().as_ref(), ctx.accounts.user.key().as_ref()],
            ctx.program_id,
        );
        user_position.bump = bump;
        user_position.owner = ctx.accounts.user.key();
        user_position.positions[0].market = market.key();
        user_position.positions[0].supply_index = market.supply_index;
        user_position.positions[0].borrow_index = market.borrow_index;
        user_position.positions[0].supply_principal = 0;
        user_position.positions[0].borrow_principal = 0;
        user_position.positions[0].is_collateral = 0;
    }

    // Compound the existing supply principal to current index before adding
    if user_position.positions[0].supply_principal > 0 {
        user_position.positions[0].supply_principal = user_position.positions[0].supply_principal * market.supply_index / user_position.positions[0].supply_index.max(1);
    }
    user_position.positions[0].supply_index = market.supply_index;

    user_position.positions[0].supply_principal += amount as u128;

    Ok(())
}



