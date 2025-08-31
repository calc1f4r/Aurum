pub const CONFIG_SEED: &str = "global_config";
pub const MINT_WHITELISTED_SEED: &str = "mint_whitelisted";
pub const MARKET_SEED: &str = "market";
pub const USER_POSITION_SEED: &str = "user_position";

// PYTH Constants

// Fixed-point scaling used for interest indices
pub const INDEX_MANTISSA: u128 = 1_000_000_000_000_000_000u128; // 1e18

// Time constants
pub const SECONDS_PER_YEAR: u128 = 31_557_600u128; // 365.25 days

// Basis points denominator (100% = 10_000 bps)
pub const BPS_DENOMINATOR: u128 = 10_000u128;