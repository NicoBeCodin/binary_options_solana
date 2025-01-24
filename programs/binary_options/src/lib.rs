use anchor_lang::{prelude::*, solana_program};
use solana_program::system_program;


declare_id!("Y9nyifuZpRfKLfKt96U7Qqtxpd7TPjDmvSNPxWVeJQN");

#[program]
pub mod binary_options {
    use super::*;


    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        strike: u64,
        expiry: i64,
        oracle_feed: Pubkey,
    ) -> Result<()> {
        // Get a mutable reference to our new Market account
        let market_account = &mut ctx.accounts.market;

        // Store the initialization parameters
        market_account.authority = ctx.accounts.authority.key();
        market_account.strike = strike;
        market_account.expiry = expiry;
        market_account.oracle_feed = oracle_feed;

        // By default, mark it as not resolved
        market_account.resolved = false;
        market_account.outcome = None;

        msg!("Market initialized successfully!");
        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(strike: u64, expiry: i64, oracle_feed: Pubkey)]
pub struct InitializeMarket<'info> {
    /// The Market account that will be created.
    /// We use `init` to allocate space and pay rent.
    /// The PDA ensures uniqueness if you want multiple markets.
    #[account(
        init,
        payer = authority,
        space = 8 + Market::LEN,  // Account discriminator + Market fields
        seeds = [
            b"market".as_ref(),
            authority.key().as_ref(),
            &strike.to_le_bytes(),
            &expiry.to_le_bytes(),
        ],
        bump
    )]
    pub market: Account<'info, Market>,

    /// The authority or creator of the market
    #[account(mut)]
    pub authority: Signer<'info>,

    /// System program required for creating accounts
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

/// The primary Market account structure.
/// This stores all relevant metadata for the binary option market.
#[account]
pub struct Market {
    /// The wallet (Pubkey) who created the market
    pub authority: Pubkey,

    /// The strike price for SOL/USD or another asset.
    pub strike: u64,

    /// Expiration timestamp (Unix time, e.g., seconds since epoch)
    pub expiry: i64,

    /// Address of the oracle feed (Chainlink or Pyth) providing price data
    pub oracle_feed: Pubkey,

    /// Whether the market has been resolved
    pub resolved: bool,

    /// Outcome after resolution:
    /// - None: Not resolved
    /// - Some(1): "Yes" outcome
    /// - Some(2): "No" outcome
    pub outcome: Option<u8>,
}

impl Market {
    /// Byte-length of the Market struct (excluding discriminator).
    /// Helps when we declare `space` in #[account(init, space = ...)]
    pub const LEN: usize = 
        // authority
        32 +
        // strike
        8 +
        // expiry
        8 +
        // oracle_feed
        32 +
        // resolved
        1 +
        // outcome: Option<u8> => 1 byte
        1;
}