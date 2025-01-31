use anchor_lang::{prelude::*, solana_program};
// use pyth_sdk_solana::{state::PriceAccount, PriceFeed}
use solana_program::system_program;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};
use anchor_spl::associated_token::AssociatedToken;


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
    //better implemenataion later
    /// Address of the oracle feed (Chainlink or Pyth) providing price data
    //pub oracle_feed: Pubkey,

    //1 for btc, 2 for sol, 3 for eth
    pub asset: u8,

    /// Whether the market has been resolved
    pub resolved: bool,

    /// Outcome after resolution:
    /// - None: Not resolved
    /// - Some(1): "Yes" outcome
    /// - Some(2): "No" outcome
    pub outcome: Option<u8>,

    pub yes_mint: Pubkey,   
    pub no_mint: Pubkey,
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
        // asset
        1 +
        // resolved
        1 +
        // outcome: Option<u8> => 1 byte
        1;
}


#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = authority.key(), // Authority who can mint
        seeds = [b"test_mint"],
        bump
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>, // Who creates & mints

    /// CHECK: The recipient must exist, but doesn’t need to sign
    pub recipient: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintOutcomeTokens<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"no_mint", market.key().as_ref()],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = user,
    )]
    pub user_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = user,
    )]
    pub user_no_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub market: Account<'info, Market>,
    /// CHECK: The Pyth price account
    pub price_account: Account<'info, PriceUpdateV2>,
}


#[derive(Accounts)]
#[instruction(strike: u64, expiry: i64, asset: u8)]
pub struct InitializeMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::LEN,
        seeds = [b"market".as_ref(), authority.key().as_ref(), &strike.to_le_bytes(), &expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump,
    )]
    pub market_vault: SystemAccount<'info>,

    // ✅ YES Token Account Validation
    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = user,
    )]
    pub user_yes_token_account: Account<'info, TokenAccount>,

    // ✅ NO Token Account Validation
    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = user,
    )]
    pub user_no_token_account: Account<'info, TokenAccount>,

    // ✅ YES Mint tied to the Market PDA
    #[account(
        mut,
        seeds = [b"yes_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    // ✅ NO Mint tied to the Market PDA
    #[account(
        mut,
        seeds = [b"no_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}





#[derive(Accounts)]
pub struct GetPriceFeed<'info> {
    #[account(mut)]
    pub payer:          Signer<'info>,
    pub price_update:   Account<'info, PriceUpdateV2>,
}



#[derive(Accounts)]
pub struct FetchCoinPrice<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    ///CHECK = The Pyth price account
    pub price_update: Account<'info, PriceUpdateV2>,
}



#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(
        init,
        payer = authority,
        space = 8,
        seeds = [b"treasury".as_ref(), authority.key().as_ref()],
        bump
    )]
    ///CHECK: The treasury account
    pub treasury: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LockFunds<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub treasury: SystemAccount<'info>, // Treasury PDA where funds will be locked

    #[account(mut)]
    pub user: Signer<'info>, // User placing the bet

    // MINT ACCOUNTS: Define the token mints (only used to verify the token type)
    #[account(
        mut,
        seeds = [b"yes_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"no_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    // TREASURY TOKEN ACCOUNTS: Where the treasury stores YES/NO tokens
    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = treasury,
    )]
    pub treasury_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = treasury,
    )]
    pub treasury_no_token_account: Account<'info, TokenAccount>,

    // USER TOKEN ACCOUNTS: Where the user will receive YES/NO tokens
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = yes_mint,
        associated_token::authority = user,
    )]
    pub user_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = no_mint,
        associated_token::authority = user,
    )]
    pub user_no_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump,
    )]
    pub market_vault: SystemAccount<'info>, // PDA for locked funds

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}



#[derive(Accounts)]
pub struct CreateOutcomeTokens<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub treasury: SystemAccount<'info>, // Treasury PDA

    #[account(
        init,
        payer = market,
        mint::decimals = 0,
        mint::authority = market,
        seeds = [b"yes_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = market,
        mint::decimals = 0,
        mint::authority = market,
        seeds = [b"no_mint".as_ref(), market.key().as_ref()],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = market,
        associated_token::mint = yes_mint,
        associated_token::authority = treasury,
    )]
    pub treasury_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = market,
        associated_token::mint = no_mint,
        associated_token::authority = treasury,
    )]
    pub treasury_no_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


