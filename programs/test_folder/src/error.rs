
use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The market has already been resolved.")]
    MarketAlreadyResolved,
    #[msg("The market has not yet expired.")]
    MarketNotExpired,
    #[msg("Invalid Pyth price account.")]
    InvalidPriceAccount,
    #[msg("Price data is unavailable.")]
    PriceUnavailable,
    #[msg("Invalid Price Feed")]
    InvalidPriceFeed,
    #[msg("Invalid Coin inputed")]
    InvalidCoin,
    #[msg("Invalid asset")]
    InvalidAsset,
    #[msg("Market not resolved")]
    MarketNotResolved,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    #[msg("Market outcome isn't resolved yet")]
    InvalidMarketOutcome
}
