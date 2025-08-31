use anchor_lang::prelude::*;
mod instructions;
mod states;
mod constants;  
mod errors;
mod utils;
use instructions::*;
use states::global_config::GlobalConfigParams;
use states::mint_whitelisted::MintWhitelistedParams;

declare_id!("FNb4MR8wChdWqh7Jkzk31VLSTXnNLbYW17j2zxyh3N6w");
#[allow(deprecated)] // suppress macro-emitted realloc deprecation until refactor
#[program]
pub mod aurum {
    use super::*;
    pub fn initialize_config(ctx: Context<InitializeConfig>, params: GlobalConfigParams) -> Result<()> {
        instructions::handler_initialize_config(ctx, params)
    }

    // set pause
    pub fn set_pause(ctx: Context<SetPause>) -> Result<()> {
        instructions::handle_set_pause(ctx)
    }

    // set unpause
    pub fn set_unpause(ctx: Context<SetPause>) -> Result<()> {
        instructions::handle_set_unpause(ctx)
    }

    // update treasury
    pub fn update_treasury(ctx: Context<UpdateTreasury>) -> Result<()> {
        instructions::handler_update_treasury(ctx)
    }

    // whitelist mint allows bot to whitelist a mint
    pub fn whitelist_mint(ctx: Context<WhitelistMint>, params: MintWhitelistedParams) -> Result<()> {
        instructions::handler_whitelist_mint(ctx, params)
    }
    
    pub fn initialize_market(ctx: Context<InitializeMarket>) -> Result<()> {
        instructions::handler_initialize_market(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::handler_deposit(ctx, amount)
    }
    
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::handler_withdraw(ctx, amount)
    }

    pub fn accrue_interest(ctx: Context<AccrueInterest>) -> Result<()> {
        instructions::handler_accrue_interest(ctx)
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        instructions::handler_repay(ctx, amount)
    }

    pub fn liquidate(ctx: Context<Liquidate>, amount: u64) -> Result<()> {
        instructions::handler_liquidate(ctx, amount)
    }
    
}


