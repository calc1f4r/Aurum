use anchor_lang::prelude::*;
use crate::states::global_config::{Config, GlobalConfigParams};

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

pub fn handler_update_config(ctx: Context<UpdateConfig>, params: GlobalConfigParams) -> Result<()> {
    ctx.accounts.config.liquidation_threshold = params.liquidation_threshold;
    ctx.accounts.config.liquidation_bonus = params.liquidation_bonus;
    ctx.accounts.config.base_rate = params.base_rate;
    ctx.accounts.config.slope1 = params.slope1;
    ctx.accounts.config.jump_slope = params.jump_slope;
    ctx.accounts.config.kink = params.kink;
    ctx.accounts.config.reserve_factor = params.reserve_factor;
    ctx.accounts.config.updated_at = Clock::get()?.unix_timestamp as u64;
    Ok(())
}