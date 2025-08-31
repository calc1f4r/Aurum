use anchor_lang::prelude::*;

use crate::constants::INDEX_MANTISSA;
use crate::errors::AurumError;
use crate::states::{market::Market, user_position::UserPosition};

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

// (Removed unused min_u128 helper)

// -----------------------------
// Balance Helpers
// -----------------------------

/// Returns (updated_supply_principal, current_supply_balance)
pub fn compute_supply_balance(user: &UserPosition, market: &Market) -> Result<(u128, u128)> {
    if user.supply_principal == 0 { return Ok((0,0)); }
    let current = user
        .supply_principal
        .checked_mul(market.supply_index)
        .ok_or(error!(AurumError::MathOverflow))?
        .checked_div(user.supply_index.max(1))
        .ok_or(error!(AurumError::MathOverflow))?;
    Ok((current, current))
}

/// Returns (updated_borrow_principal, current_borrow_balance)
pub fn compute_borrow_balance(user: &UserPosition, market: &Market) -> Result<(u128, u128)> {
    if user.borrow_principal == 0 { return Ok((0,0)); }
    let current = user
        .borrow_principal
        .checked_mul(market.borrow_index)
        .ok_or(error!(AurumError::MathOverflow))?
        .checked_div(user.borrow_index.max(1))
        .ok_or(error!(AurumError::MathOverflow))?;
    Ok((current, current))
}

// (Removed unused get_user_balances helper to avoid dead_code warnings)


