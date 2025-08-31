use anchor_lang::prelude::*;

use crate::constants::{BPS_DENOMINATOR, INDEX_MANTISSA, SECONDS_PER_YEAR};
use crate::states::global_config::Config;
use crate::states::market::Market;
use crate::utils::{mul_div_floor, ray_div, ray_mul};

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        constraint = !config.paused,
    )]
    pub config: Account<'info, Config>,
}

pub fn handler_accrue_interest(ctx: Context<AccrueInterest>) -> Result<()> {
    accrue_market(&mut ctx.accounts.market, &ctx.accounts.config)
}

pub fn accrue_market(market: &mut Account<Market>, config: &Config) -> Result<()> {
    let now_ts = Clock::get()?.unix_timestamp as u64;
    if now_ts <= market.last_updated_ts { return Ok(()); }
    let dt_seconds: u128 = (now_ts - market.last_updated_ts) as u128;

    let total_cash: u128 = market.total_cash;
    let total_borrows: u128 = market.total_borrows;
    let total_reserves: u128 = market.total_reserves;

    // Utilization = borrows / (cash + borrows - reserves)
    let mut denominator = total_cash + total_borrows;
    if denominator < total_reserves { denominator = 0; } else { denominator -= total_reserves; }
    let utilization = if denominator == 0 || total_borrows == 0 {
        0u128
    } else {
        ray_div(total_borrows, denominator)?
    };

    // Convert kink and reserve_factor from BPS to 1e18 mantissa
    let kink_mantissa: u128 = (config.kink as u128) * (INDEX_MANTISSA / BPS_DENOMINATOR);
    let reserve_factor_bps: u128 = config.reserve_factor as u128;
    let reserve_factor_mantissa: u128 = reserve_factor_bps * (INDEX_MANTISSA / BPS_DENOMINATOR);

    // Interest rate model (per year), all mantissa 1e18
    let base_rate: u128 = config.base_rate as u128;
    let slope1: u128 = config.slope1 as u128;
    let jump_slope: u128 = config.jump_slope as u128;

    let borrow_rate_per_year = if utilization <= kink_mantissa {
        let u_over_k = if kink_mantissa == 0 { 0 } else { ray_div(utilization, kink_mantissa)? };
    base_rate + ray_mul(slope1, u_over_k)?
    } else {
    let above_k = utilization - kink_mantissa;
    let one_minus_k = INDEX_MANTISSA - kink_mantissa;
        let frac = if one_minus_k == 0 { 0 } else { ray_div(above_k, one_minus_k)? };
    base_rate + slope1 + ray_mul(jump_slope, frac)?
    };

    // Convert annual rate to per-second
    let borrow_rate_per_sec = borrow_rate_per_year / SECONDS_PER_YEAR;

    // Borrow factor over dt: 1 + r_sec * dt
    let rsec_times_dt = borrow_rate_per_sec * dt_seconds;
    let borrow_factor = INDEX_MANTISSA + rsec_times_dt;

    // Update borrow index and total borrows
    let old_borrow_index = market.borrow_index;
    let old_total_borrows = total_borrows;
    let new_borrow_index = mul_div_floor(old_borrow_index, borrow_factor, INDEX_MANTISSA)?;
    let new_total_borrows = mul_div_floor(old_total_borrows, borrow_factor, INDEX_MANTISSA)?;

    let interest_accumulated = new_total_borrows - old_total_borrows;

    // Add to reserves
    let reserves_add = mul_div_floor(interest_accumulated, reserve_factor_bps, BPS_DENOMINATOR)?;
    let new_total_reserves = market.total_reserves + reserves_add;

    // Supply rate: r_borrow * U * (1 - reserve_factor)
    let one_minus_rf = INDEX_MANTISSA - reserve_factor_mantissa;
    let supply_rate_per_year = ray_mul(ray_mul(borrow_rate_per_year, utilization)?, one_minus_rf)?;
    let supply_rate_per_sec = supply_rate_per_year / SECONDS_PER_YEAR;
    let ssec_times_dt = supply_rate_per_sec * dt_seconds;
    let supply_factor = INDEX_MANTISSA + ssec_times_dt;
    let new_supply_index = mul_div_floor(market.supply_index, supply_factor, INDEX_MANTISSA)?;

    market.borrow_index = new_borrow_index;
    market.total_borrows = new_total_borrows;
    market.total_reserves = new_total_reserves;
    market.supply_index = new_supply_index;
    market.last_updated_ts = now_ts;

    Ok(())
}


