pub mod health_factor;

use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2};

use crate::constants::INDEX_MANTISSA;
use crate::errors::AurumError;
use crate::states::{market::Market, user_position::{UserPosition, Position}};

#[inline]
pub fn ray_mul(a: u128, b: u128) -> Result<u128> {
    a.checked_mul(b)
        .and_then(|x| x.checked_div(INDEX_MANTISSA))
        .ok_or(error!(crate::errors::AurumError::MathOverflow))
}

#[inline]
pub fn ray_div(a: u128, b: u128) -> Result<u128> {
    if b == 0 { return Err(error!(crate::errors::AurumError::MathOverflow)); }
    a.checked_mul(INDEX_MANTISSA)
        .and_then(|x| x.checked_div(b))
        .ok_or(error!(crate::errors::AurumError::MathOverflow))
}

#[inline]
pub fn mul_div_floor(a: u128, b: u128, denom: u128) -> Result<u128> {
    if denom == 0 { return Err(error!(crate::errors::AurumError::MathOverflow)); }
    a.checked_mul(b)
        .and_then(|x| x.checked_div(denom))
        .ok_or(error!(crate::errors::AurumError::MathOverflow))
}

// Helper to find position for a market
pub fn find_position(user: &UserPosition, market_key: Pubkey) -> Option<&Position> {
    user.positions.iter().find(|p| p.market == market_key && p.is_active())
}

// -----------------------------
// Balance Helpers
// -----------------------------

/// Returns (updated_supply_principal, current_supply_balance)
pub fn compute_supply_balance(user: &UserPosition, market: &anchor_lang::prelude::Account<Market>) -> Result<(u128, u128)> {
    let position = find_position(user, market.key()).ok_or(error!(AurumError::InvalidPosition))?;
    if position.supply_principal == 0 { return Ok((0,0)); }
    let current = position
        .supply_principal
        .checked_mul(market.supply_index)
        .ok_or(error!(AurumError::MathOverflow))?
        .checked_div(position.supply_index.max(1))
        .ok_or(error!(AurumError::MathOverflow))?;
    Ok((current, current))
}

/// Returns (updated_borrow_principal, current_borrow_balance)
pub fn compute_borrow_balance(user: &UserPosition, market: &anchor_lang::prelude::Account<Market>) -> Result<(u128, u128)> {
    let position = find_position(user, market.key()).ok_or(error!(AurumError::InvalidPosition))?;
    if position.borrow_principal == 0 { return Ok((0,0)); }
    let current = position
        .borrow_principal
        .checked_mul(market.borrow_index)
        .ok_or(error!(AurumError::MathOverflow))?
        .checked_div(position.borrow_index.max(1))
        .ok_or(error!(AurumError::MathOverflow))?;
    Ok((current, current))
}

// (Removed unused get_user_balances helper to avoid dead_code warnings)

/// Get price value from Pyth price feed
pub fn get_price_value(price_update: &PriceUpdateV2, feed_id: &[u8; 32], maximum_age: u64) -> Result<u128> {
    let clock = Clock::get()?;
    let price = price_update.get_price_no_older_than(&clock, maximum_age, feed_id)
        .map_err(|_| error!(AurumError::InvalidPriceFeed))?;

    // Convert price to u128 with proper scaling
    // Price is returned as (price ± conf) * 10^exponent
    // We want to normalize to 18 decimal places for consistency
    let price_value = if price.exponent >= 0 {
        // Price is already in whole units, scale up to 18 decimals
        (price.price as u128) * 10u128.pow(price.exponent as u32) * 10u128.pow(18 - price.exponent as u32)
    } else {
        // Price has decimal places, scale to 18 decimals
        (price.price as u128) * 10u128.pow(18 + (-price.exponent) as u32)
    };

    Ok(price_value)
}

/// Convert asset amount to USD value using price feed
pub fn get_asset_value_in_usd(amount: u128, price_update: &PriceUpdateV2, feed_id: &[u8; 32], maximum_age: u64) -> Result<u128> {
    let price = get_price_value(price_update, feed_id, maximum_age)?;
    // Amount is in base units, price is normalized to 18 decimals
    // Result is in USD with 18 decimals
    mul_div_floor(amount, price, 10u128.pow(18))
}
