use anchor_lang::prelude::*;

use solana_program::{pubkey, pubkey::Pubkey};
use solana_program::system_program;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3, MasterEditionAccount, Metadata},
    token::{burn, mint_to, Burn, Mint, MintTo, Token, TokenAccount},
};

use mpl_token_metadata::accounts::MasterEdition;
use mpl_token_metadata::{types::DataV2};
use mpl_token_metadata::ID as METAPLEX_PROGRAM_ID;

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
        2;

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
pub struct Redeem<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.authority.as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Box<Account<'info, Market>>, // ✅ Market PDA holds the locked lamports

    #[account(
        mut,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump
    )]
    pub yes_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [b"no_mint", market.key().as_ref()],
        bump
    )]
    pub no_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = market,
    )]
    pub treasury_yes_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = market,
    )]
    pub treasury_no_token_account: Box<Account<'info, TokenAccount>>,

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
}


#[derive(Accounts)]
pub struct LockFunds<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.authority.as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump
    )]
    pub yes_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [b"no_mint", market.key().as_ref()],
        bump
    )]
    pub no_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = market,
    )]
    pub treasury_yes_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = market,
    )]
    pub treasury_no_token_account: Box<Account<'info, TokenAccount>>,

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

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    
}

//ADMIN STUFF 

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
pub struct InitializeOutcomeMints<'info> {
    #[account(
        mut,
        seeds = [b"market".as_ref(), market.authority.key().as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        mint::decimals = 0,
        mint::authority = market,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump
    )]
    pub yes_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = authority,
        mint::decimals = 0,
        mint::authority = market,
        seeds = [b"no_mint", market.key().as_ref()],
        bump
    )]
    pub no_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

const METADATA_PROGRAM_ID: Pubkey=  pubkey!(Pubkey::new_from_array([11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70]));

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(
        mut,
        seeds = [b"market".as_ref(), market.authority.key().as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
 
    // The PDA is both the address of the mint account and the mint authority
    #[account(
        init,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump,
        payer = authority,
        mint::decimals = 0,
        mint::authority = yes_mint,
    )]
    pub yes_mint: Account<'info, Mint>,
 
    ///CHECK: Using "address" constraint to validate metadata account address
    #[account(mut,
        address = Pubkey::find_program_address(
            &[b"metadata".as_ref(), METAPLEX_PROGRAM_ID.as_ref(), yes_mint.key().as_ref()],
            &METADATA_PROGRAM_ID,
        ).0)]
    pub metadata_account: UncheckedAccount<'info>,
 
    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintMetadataTokens<'info> {
    #[account(
        mut,
        seeds = [b"market".as_ref(), market.authority.key().as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump,
        mint::authority = yes_mint,
    )]
    pub yes_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = yes_mint,
        associated_token::authority = market,
    )]
    pub treasury_yes_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeTreasuryTokenAccounts<'info> {
    #[account(
        mut,
        seeds = [b"market".as_ref(), market.authority.key().as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub authority: Signer<'info>,

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
        init_if_needed,
        payer = authority,
        associated_token::mint = yes_mint,
        associated_token::authority = market,
    )]
    pub treasury_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = no_mint,
        associated_token::authority = market,
    )]
    pub treasury_no_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct MintOutcomeTokens<'info> {
    #[account(
        mut,
        seeds = [b"market".as_ref(), market.authority.key().as_ref(), &market.strike.to_le_bytes(), &market.expiry.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub authority: Signer<'info>,

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
        associated_token::authority = market,
    )]
    pub treasury_yes_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = market,
    )]
    pub treasury_no_token_account: Account<'info, TokenAccount>,


    pub token_program: Program<'info, Token>,
}
