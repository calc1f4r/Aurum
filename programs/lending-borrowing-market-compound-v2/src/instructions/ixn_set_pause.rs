use anchor_lang::prelude::*;
use crate::states::global_config::Config;
use crate::constants::CONFIG_SEED;
use crate::errors::AurumError;
#[derive(Accounts)]
pub struct SetPause<'info> {
    #[account(
        mut,
        has_one = admin,
        seeds = [CONFIG_SEED.as_bytes()],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    // only state update no need of the system program
}

pub fn handle_set_pause(ctx: Context<SetPause>) -> Result<()> {
    require!(!ctx.accounts.config.paused, AurumError::ConfigAlreadyPaused);
    ctx.accounts.config.paused = true;
    ctx.accounts.config.updated_at = Clock::get()?.unix_timestamp as u64;
    Ok(())
}

pub fn handle_set_unpause(ctx: Context<SetPause>) -> Result<()> {
    require!(ctx.accounts.config.paused, AurumError::ConfigAlreadyUnpaused);
    ctx.accounts.config.paused = false;
    ctx.accounts.config.updated_at = Clock::get()?.unix_timestamp as u64;
    Ok(())
}