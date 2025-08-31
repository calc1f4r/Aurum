use super::{UserPosition, Market};
use crate::states::{global_config::GlobalConfig, user_position::Position};
use anchor_lang::prelude::*;
use std::convert::TryInto;

pub fn calculate_health_factor(
    user_position: &UserPosition,
    markets: &[Account<Market>],
    global_config: &GlobalConfig,
) -> Result<u64> {
    let mut total_collateral_value = 0;
    let mut total_borrow_value = 0;

    for position in user_position.positions.iter().filter(|p| p.is_active()) {
        let market = markets
            .iter()
            .find(|m| m.key() == position.market)
            .ok_or(ProgramError::Custom(1))?; // Custom error for market not found

        let (collateral_value, borrow_value) =
            calculate_position_value(position, market, global_config)?;
        total_collateral_value += collateral_value;
        total_borrow_value += borrow_value;
    }

    if total_borrow_value == 0 {
        // Infinite health factor, or a very large number to represent it
        return Ok(u64::MAX);
    }

    let health_factor = (total_collateral_value * 100) / total_borrow_value;
    Ok(health_factor.try_into().unwrap())
}

fn calculate_position_value(
    position: &Position,
    market: &Account<Market>,
    global_config: &GlobalConfig,
) -> Result<(u128, u128)> {
    // These would be derived from Pyth or another oracle
    let asset_price = get_asset_price(&market.mint, global_config)?;
    let ltv = market.ltv_bps as u128; // in basis points

    // Calculate current supply amount with accrued interest
    let current_supply = if position.supply_index > 0 {
        (position.supply_principal * market.supply_index) / position.supply_index
    } else {
        position.supply_principal
    };

    // Calculate current borrow amount with accrued interest
    let current_borrow = if position.borrow_index > 0 {
        (position.borrow_principal * market.borrow_index) / position.borrow_index
    } else {
        position.borrow_principal
    };

    let supply_value = (current_supply * asset_price) / 10u128.pow(market.decimals as u32);
    let borrow_value = (current_borrow * asset_price) / 10u128.pow(market.decimals as u32);

    let collateral_value = if position.is_collateral != 0 {
        (supply_value * ltv) / 10000 // ltv in basis points
    } else {
        0
    };

    Ok((collateral_value, borrow_value))
}

// Placeholder for oracle price fetching
fn get_asset_price(
    _asset_mint: &Pubkey,
    _global_config: &GlobalConfig,
) -> Result<u128> {
    // In a real implementation, this would fetch from an oracle like Pyth
    // For now, using a mock price
    Ok(100_000_000) // Mock price, e.g., $100 in USD with 8 decimals or adjust
}
