use anchor_lang::prelude::*;
use crate::states::global_config::GlobalConfig;
use crate::errors::AurumError::TresurySameAsOldOne;
#[derive(Accounts)]
pub struct UpdateTreasury<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, GlobalConfig>,
    /// CHECK : CHECHED AND ADMIN CONTROLLED 
    #[account(constraint = new_treasury.key() != config.treasury @TresurySameAsOldOne)]
    pub new_treasury: AccountInfo<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

pub fn handler_update_treasury(ctx: Context<UpdateTreasury>) -> Result<()> {
    ctx.accounts.config.treasury = ctx.accounts.new_treasury.key();
    Ok(())
}