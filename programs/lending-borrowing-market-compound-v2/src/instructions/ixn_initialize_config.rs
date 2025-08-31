use anchor_lang::prelude::*;
use crate::constants::CONFIG_SEED;
use crate::states::global_config::{Config};

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

pub fn handler_initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
    ctx.accounts.config.bump = ctx.bumps.config;
    ctx.accounts.config.admin = ctx.accounts.admin.key();
    ctx.accounts.config.paused = false;
    Ok(())
}