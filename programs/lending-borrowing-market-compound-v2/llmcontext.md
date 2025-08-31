go Compound-Style Lending Market on Solana
A Compound v2–style Solana lending protocol consists of a global market config, per-asset Reserves, and per-user Obligations. The global LendingMarket state (a PDA account) tracks the program parameters and authorities. For example, the LendingMarket struct may include fields like: owner (authority to add reserves), quote_currency (e.g. "USD"), and the program IDs for SPL tokens and the Pyth oracle[1][2]. The LendingMarket is initialized once with an instruction (e.g. InitLendingMarket) that sets the owner, quote currency, and which token/oracle programs to use.
The program allows creating multiple Reserves, one for each asset (token) that can be lent or borrowed. Each Reserve tracks the pool of supplied liquidity (the underlying asset) and the corresponding collateral token (a “cToken” SPL mint). A Reserve account (also a PDA) might be structured as:
pub struct Reserve {
    pub version: u8,
    pub last_update: LastUpdate,
    pub lending_market: Pubkey,
    pub liquidity: ReserveLiquidity,
    pub collateral: ReserveCollateral,
    pub config: ReserveConfig,
}
Here, ReserveLiquidity holds info about the underlying asset (its mint, vault, and amounts) and oracle price, and ReserveCollateral holds info about the collateral SPL mint and supply. For example, the ReserveLiquidity struct might include[3]:
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,         // Underlying token mint
    pub mint_decimals: u8,
    pub supply_pubkey: Pubkey,       // Vault holding underlying tokens
    pub pyth_oracle_pubkey: Pubkey,  // Pyth price account for this token
    pub switchboard_oracle_pubkey: Pubkey,
    pub available_amount: u64,       // Tokens in the vault (not borrowed)
    pub borrowed_amount_wads: Decimal,      // Total borrowed (scaled)
    pub cumulative_borrow_rate_wads: Decimal, // Accrued interest factor
    pub market_price: Decimal,       // Last fetched oracle price
}
The ReserveConfig (not shown above) would hold parameters like collateral factor (LTV), liquidation threshold, reserve factor, and interest-rate model parameters (base rate, slopes, kink). These govern how much can be borrowed and how interest accrues. All prices are treated in a common quote currency (e.g. USD); stablecoins simply have prices ≈1.0 in that quote.
Each user has an Obligation account recording their deposits and borrows across reserves. An Obligation struct looks like[4][5]:
pub struct Obligation {
    pub version: u8,
    pub last_update: LastUpdate,
    pub lending_market: Pubkey,
    pub owner: Pubkey,                     // user pubkey
    pub deposits: Vec<ObligationCollateral>, // collateral by reserve
    pub borrows: Vec<ObligationLiquidity>,   // borrow by reserve
    pub deposited_value: Decimal,          // total collateral value (USD)
    pub borrowed_value: Decimal,           // total borrow value (USD)
    pub allowed_borrow_value: Decimal,     // collateral * LTV
    pub unhealthy_borrow_value: Decimal,   // collateral * liquidationThreshold
}
The Obligation fields deposits and borrows list amounts per reserve. The deposited_value and borrowed_value are the USD values (sum of each asset amount times its price). The allowed_borrow_value is computed as the weighted collateral sum times the per-asset loan-to-value (LTV) factors. If borrowed_value > allowed_borrow_value, the position is underwater (shortfall) and subject to liquidation[6].
Interest-Rate Model (Jump Rate)
Each reserve uses a jump-rate interest model like Compound v2. Define the utilization rate U of a reserve as:
U="totalBorrows" /("cash" +"totalBorrows" -"reserves" ) ,
where “cash” is the available_amount in the vault, “totalBorrows” is borrowed_amount_wads, and “reserves” is any portion kept by protocol. (All values are scaled to a common 10¹⁸ precision[7].)
The borrow interest rate is a piecewise linear function of U [8][9]:
r_b (U)={■(a_1 U+b,&U<"kink" ,@a_1 "kink" +a_2 (U-"kink" )+b,&U≥"kink" .)┤
Here b is the base rate (y-intercept), a_1 the slope up to the “kink” point, and a_2 the higher slope beyond the kink[8]. These parameters (baseRate, multiplier, jumpMultiplier, kink) are stored in the reserve’s config and can be updated by the owner. Interest is typically measured per year or per second and converted to per-slot accrual.
The supply interest rate (what lenders earn) is derived from the borrow rate. In a simple model, supplyRate = U×r_b (U)×(1-"reserveFactor" ) [10]. In other words, lenders earn interest proportional to the utilization and the borrow rate, after reserving a protocol fee. (For example, if utilization is 50% and borrow rate is 10% APR, with reserve factor 10%, then supply rate ≈0.5 × 10% × 0.9 = 4.5% APR.) Interest accrues continuously (or per slot) by updating the reserve’s cumulative borrow rate each time any user interacts. In practice, on each operation the program calculates the new borrow index:
"newBorrowIndex"="oldBorrowIndex"×(1+r_b×Δt) ,
and scales total borrows accordingly. This is analogous to Compound v2’s accrueInterest mechanism[11].
Supply (Deposit) and Withdraw (Redeem)
When a user supplies (deposits) an asset to a reserve, they send underlying tokens to the reserve’s vault. In exchange, the protocol mints collateral tokens (like cTokens) to the user. The number of collateral tokens minted is based on the exchange rate, which represents the ratio between total assets and total collateral supply. For example, Compound uses the formula[12]:
"exchangeRate"=("totalCash" +"totalBorrows" -"totalReserves" )/"totalCollateralSupply" .
On deposit, the user’s collateral minted = depositedAmount / exchangeRate. In our design, we store analogous values so that minting stays consistent. The liquidity.cumulative_borrow_rate_wads effectively incorporates interest so that the exchange rate updates correctly.
Conversely, when a user withdraws (redeems) collateral, they burn some of their collateral tokens and receive underlying tokens back from the vault. The underlying amount received = collateralBurned × current exchangeRate. All deposits and withdrawals automatically trigger interest accrual first. As Compound’s docs explain, the exchange rate increases over time as interest accrues[12], so lenders’ balances grow.
Borrow and Repay
To borrow, a user must first have sufficient deposited collateral. The program checks that after borrowing the user’s obligation remains above the liquidation threshold. In practice, this means:
〖"borrowed_value" 〗_"new" =〖"borrowed_value" 〗_"old" +〖"price" 〗_"borrow" ×"amount",
and it must satisfy
〖"borrowed_value" 〗_"new"  ≤ "allowed_borrow_value"=∑_i^▒(〖"collateral_amount" 〗_i×P_i×〖"LTV" 〗_i ) .
In other words, the total borrowed value in USD must not exceed the collateral’s USD value times its loan-to-value (LTV) factors[6]. If the check passes, the user’s borrows entry for that asset is increased (taking into account accrued interest), the reserve’s borrowed_amount_wads is updated, and the underlying tokens are transferred to the user. Interest then continues to accrue on this new borrowed balance.
On repayment, the user sends the borrowed asset back to the vault. The program subtracts the amount (plus any accrued interest) from the user’s borrows and from borrowed_amount_wads. If the user repays more than owed, the excess can be refunded. Any repaid amount also contributes to the reserve’s liquidity (available_amount increases). Both borrow and repay operations should first call an interest accrual step to update rates as described above[11].
Liquidation (Partial Allowed)
If a user’s health falls below 1 (i.e. borrowed_value exceeds allowed borrow), their position is undercollateralized. Anyone can liquidate such a position. The procedure is:
	Identify that the user has a “shortfall”: excess borrow beyond collateral limits[6].
	The liquidator chooses one borrow asset to repay. Compound v2 limits this by a close factor (e.g. 50% per liquidation)[13]. This means the liquidator can repay up to (closeFactor × the user’s borrowed amount) in one go.
	Upon repayment of repayAmount, the liquidator seizes collateral. The collateral amount is computed using the liquidation incentive. If the liquidationIncentive is 1.08 (8% bonus), then each 1 USD of debt repaid entitles the liquidator to 1.08 USD of collateral value. Concretely:
"collateral_seized"="repayAmount" × 〖"price" 〗_"borrow" /〖"price" 〗_"collateral"   × "liquidationIncentive".
This follows Compound’s rule that “liquidationIncentive … multiplied by the closed borrow amount … determines how much collateral can be seized”[14]. (A portion of the bonus may be sent to protocol reserves, per config.)
	The user’s borrows entry is reduced by repayAmount, and their collateral entries are reduced by collateral_seized. The liquidator receives the seized collateral tokens. The user’s obligation values are recomputed; ideally this one liquidation brings their health factor above the threshold. Multiple partial liquidations are possible until the account is healthy.
In summary, partial liquidation repays part of the debt and transfers a proportionally larger collateral, thereby lowering the loan-to-value back into a safe range. The design should enforce that after liquidation: borrowed_value_new ≤ unhealthy_borrow_value (the liquidation threshold), restoring solvency.
Pyth Oracle Pricing
All asset values are computed in a common quote currency using on-chain oracles. Each Reserve stores a pyth_oracle_pubkey (a Pyth price account)[3]. At each accrual or action, the program reads the latest price from that Pyth account (in USD or another quote), adjusting for token decimals. The USD value of any collateral or borrow is then amount * price. These prices feed into the LTV and health calculations above[6]. For example, after a supply or borrow, the protocol updates the user’s deposited_value and borrowed_value by summing (amount * oracle_price) across all assets, multiplied by their collateral factors or 1. This ensures users can only borrow up to (collateral * LTV) as per the current market prices.
Instructions and Workflow
Putting it all together, the program would expose instructions such as: - InitLendingMarket: create the global market account (set owner, quote currency, program IDs). - InitReserve: initialize a new Reserve (set the underlying mint, collateral mint, oracle key, and config parameters like LTV, reserve factor, interest rates). - DepositReserveLiquidity / MintCollateral: user deposits tokens into a Reserve vault and receives collateral tokens (two-step: deposit to reserve, then deposit to obligation). - RedeemReserveCollateral / WithdrawLiquidity: user burns collateral tokens and withdraws underlying from the vault. - BorrowObligationLiquidity: user borrows from a reserve against their obligation’s collateral. - RepayObligationLiquidity: user repays borrowed tokens to reduce debt. - LiquidateObligation: liquidator repays a user’s debt and seizes collateral.
Each of these operations would update the corresponding state: accrues interest, adjusts available_amount, borrowed_amount_wads, and user Obligation balances. The formulas above ensure that loans stay within safe limits and that interest and liquidation behave correctly.
Sources: The account structures above follow the SPL Lending program design[1][3]. The utilization and interest formulas come from Compound’s model[8][10], and liquidation logic follows Compound v2’s rules[6][14]. All calculations use Pyth oracle prices as the on-chain price source.
________________________________________
[1] [2] [3] [4] [5] Solana account structure | Fluidity Money (EN)
https://docs.fluidity.money/docs/developers/solana-account-structure
[6] [13] [14] Compound v2 Docs | Comptroller
https://docs.compound.finance/v2/comptroller/
[7] [9] [10] [12] SlowMist: Compound Finance V2 Security Audit Manual | by SlowMist | Medium
https://slowmist.medium.com/slowmist-compound-finance-v2-security-audit-manual-3ad56bd596da
[8] BaseJumpRateModelV2 | Venus Protocol
https://docs-v4.venus.io/technical-reference/reference-isolated-pools/interest-rate-models/base-jump-rate-model-v2
[11] Compound v2 Documentation
https://docs.compound.finance/v2/
