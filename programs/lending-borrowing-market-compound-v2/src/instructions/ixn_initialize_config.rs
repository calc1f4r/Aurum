use anchor_lang::prelude::*;
use crate::constants::CONFIG_SEED;
use crate::states::global_config::{Config, GlobalConfigParams};

// initialized the global config 
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED.as_bytes()],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECKED : THIS ACCOUNT IS CHECHED
    pub treasury: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler_initialize_config(ctx: Context<InitializeConfig>, params: GlobalConfigParams) -> Result<()> {
    ctx.accounts.config.bump = ctx.bumps.config;
    ctx.accounts.config.admin = ctx.accounts.admin.key();
    ctx.accounts.config.treasury = ctx.accounts.treasury.key();
    ctx.accounts.config.paused = false;
    ctx.accounts.config.updated_at = Clock::get()?.unix_timestamp as u64;
    
    // Set interest rate model parameters
    ctx.accounts.config.liquidation_threshold = params.liquidation_threshold;
    ctx.accounts.config.liquidation_bonus = params.liquidation_bonus;
    ctx.accounts.config.base_rate = params.base_rate;
    ctx.accounts.config.slope1 = params.slope1;
    ctx.accounts.config.jump_slope = params.jump_slope;
    ctx.accounts.config.kink = params.kink;
    ctx.accounts.config.reserve_factor = params.reserve_factor;
    
    Ok(())
}