use anchor_lang::{
    prelude::*,
    solana_program::program::invoke_signed,
    solana_program::system_instruction,
    solana_program::pubkey::Pubkey,
};

use anchor_spl::metadata::mpl_token_metadata::types::DataV2;
use anchor_spl::token_2022::spl_token_2022::extension::token_metadata;
use anchor_spl::token_2022::spl_token_2022::solana_zk_token_sdk::zk_token_proof_program;
// use SolanaPriceAccount::account_to_feed;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use pyth_solana_receiver_sdk::price_update::get_feed_id_from_hex;
use crate::state::*;
use crate::error::ErrorCode;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{ create_metadata_accounts_v3, Metadata, CreateMetadataAccountsV3 },
    token::{ self, MintTo, Burn, Transfer, CloseAccount },
};
use mpl_token_metadata::types::DataV2 as MetaDataV2;
use mpl_token_metadata::accounts::MasterEdition;

use mpl_token_metadata::ID as METAPLEX_PROGRAM_ID;

pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // Ensure the market has not already been resolved
    if market.resolved {
        msg!("Market is already resolved.");
        return Err(ErrorCode::MarketAlreadyResolved.into());
    }
    // Ensure the market has expired
    let current_time = Clock::get()?.unix_timestamp;
    if current_time < market.expiry {
        msg!(
            "Market has not expired yet. Current time: {}, Expiry: {}",
            current_time,
            market.expiry
        );
        return Err(ErrorCode::MarketNotExpired.into());
    }
    msg!("Resolving market...");
    msg!("Strike price is {}", market.strike);

    // Fetch price for the associated asset
    msg!("Fetching price for asset: {}", market.asset);
    let price = match market.asset {
        1 => fetch_btc_price(&ctx.accounts.price_account)?, // BTC
        2 => fetch_sol_price(&ctx.accounts.price_account)?, // SOL
        3 => fetch_eth_price(&ctx.accounts.price_account)?, // ETH
        _ => {
            msg!("Invalid asset type: {}", market.asset);
            return Err(ErrorCode::InvalidAsset.into());
        }
    };
    msg!("Fetched price: {}", price);
    // Determine the outcome based on the strike price
    if price >= (market.strike as f64) {
        market.outcome = Some(1); // "Yes"
        msg!("Outcome: Yes (price >= strike)");
    } else {
        market.outcome = Some(2); // "No"
        msg!("Outcome: No (price < strike)");
    }

    // Mark the market as resolved
    market.resolved = true;
    msg!("Market resolved successfully with outcome: {:?}", market.outcome);

    Ok(())
}

const ADMIN_KEY: &str = "EJZQiTeikeg8zgU7YgRfwZCxc9GdhTsYR3fQrXv3uK9V";
const LAMPORTS_PER_TOKEN: u64 = 100_000;

pub fn lock_funds(ctx: Context<LockFunds>, amount: u64) -> Result<()> {
    let lamports_to_lock = amount * LAMPORTS_PER_TOKEN;
    let market_seeds = &[
        b"market",
        ctx.accounts.market.authority.as_ref(),
        &ctx.accounts.market.strike.to_le_bytes(),
        &ctx.accounts.market.expiry.to_le_bytes(),
        &[ctx.bumps.market],
    ];
    let signer_seeds = &[&market_seeds[..]];

    // ✅ Use invoke_signed to transfer lamports
    invoke_signed(
        &system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.market.key(),
            lamports_to_lock
        ),
        &[
            ctx.accounts.user.to_account_info(),
            ctx.accounts.market.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds // ✅ Sign the transaction with the market PDA seeds
    )?;

    msg!(
        "User locked {} lamports and received {} YES and NO tokens each.",
        lamports_to_lock,
        amount
    );

    // ✅ Transfer YES tokens
    let yes_transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.treasury_yes_token_account.to_account_info(),
            to: ctx.accounts.user_yes_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer_seeds
    );
    token::transfer(yes_transfer_ctx, amount)?;

    // ✅ Transfer NO tokens
    let no_transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.treasury_no_token_account.to_account_info(),
            to: ctx.accounts.user_no_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer_seeds
    );
    token::transfer(no_transfer_ctx, amount)?;

    Ok(())
}

pub fn redeem(ctx: Context<Redeem>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    // let market_authority = &mut ctx.accounts.market_authority;
    let user = &ctx.accounts.user;
    let token_program = &ctx.accounts.token_program;

    // ✅ Ensure the market has been resolved
    require!(market.resolved, ErrorCode::MarketNotResolved);

    // ✅ Determine which token should be burned and redeemed
    let (user_token_account, treasury_token_account, mint) = match market.outcome {
        Some(1) => {
            msg!("✅ Market outcome is YES. Burning all YES tokens.");
            (
                &ctx.accounts.user_yes_token_account,
                &ctx.accounts.treasury_yes_token_account,
                &ctx.accounts.yes_mint,
            )
        }
        Some(2) => {
            msg!("✅ Market outcome is NO. Burning all NO tokens.");
            (
                &ctx.accounts.user_no_token_account,
                &ctx.accounts.treasury_no_token_account,
                &ctx.accounts.no_mint,
            )
        }
        _ => {
            return Err(ErrorCode::MarketNotResolved.into());
        }
    };

    // ✅ Fetch the user's token balance
    let user_token_balance = user_token_account.amount;
    require!(user_token_balance > 0, ErrorCode::InsufficientTokens);

    let total_lamports = user_token_balance
        .checked_mul(LAMPORTS_PER_TOKEN)
        .ok_or(ErrorCode::Overflow)?;

    // ✅ Burn all user's tokens
    let cpi_accounts = Burn {
        mint: mint.to_account_info(),
        from: user_token_account.to_account_info(),
        authority: user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::burn(cpi_ctx, user_token_balance)?;

    msg!(
        "✅ Burned {} tokens for user. Transferring {} lamports...",
        user_token_balance,
        total_lamports
    );

    // ✅ Transfer lamports from Market PDA to the user
    let market_seeds = &[
        b"market",
        market.authority.as_ref(),
        &market.strike.to_le_bytes(),
        &market.expiry.to_le_bytes(),
        &[ctx.bumps.market],
    ];
    let signer = &[&market_seeds[..]];
    **ctx.accounts.market.to_account_info().try_borrow_mut_lamports()? -= total_lamports;
    **ctx.accounts.user.try_borrow_mut_lamports()? += total_lamports;
    msg!(
        "✅ Successfully redeemed {} tokens and transferred {} lamports to user",
        user_token_balance,
        total_lamports
    );

    // Close the user's token account to reclaim rent
    let close_cpi_accounts = CloseAccount {
        account: user_token_account.to_account_info(),
        destination: user.to_account_info(),
        authority: user.to_account_info(),
    };
    let close_cpi_ctx = CpiContext::new(token_program.to_account_info(), close_cpi_accounts);
    token::close_account(close_cpi_ctx)?;

    msg!("✅ Closed user's token account to reclaim rent.");

    Ok(())
}

pub const MAXIMUM_AGE: u64 = 3600; // 1 hour
pub const FEED_ID: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
pub const STALENESS_THRESHOLD: u64 = 120; // staleness threshold in seconds

//Doesnt work
pub fn get_price_feed(ctx: Context<GetPriceFeed>, feed_id_str: String) -> Result<f64> {
    msg!("get_price_feed: feed_id_str: {}", feed_id_str);

    let price_update = &mut ctx.accounts.price_update;

    let feed_id = get_feed_id_from_hex(&feed_id_str.as_str())?;
    let price = price_update.get_price_no_older_than(
        &Clock::get()?,
        STALENESS_THRESHOLD,
        &feed_id
    )?;

    let final_price = (price.price as f64) * (10f64).powi(price.exponent as i32);

    msg!("get_price_feed feed_id {:?}, price is {}", feed_id, final_price);

    Ok(final_price)
}

pub fn fetch_btc_price(price_account: &Account<PriceUpdateV2>) -> Result<f64> {
    // 1-Fetch latest price

    // get_price_no_older_than will fail if the price update is for a different price feed.
    // This string is the id of the BTC/USD feed. See https://pyth.network/developers/price-feed-ids for all available IDs.
    let feed_id: [u8; 32] = get_feed_id_from_hex(
        "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"
    )?;
    msg!("btc feed id: {:?}", feed_id);

    let price = price_account.get_price_no_older_than(
        &Clock::get()?,
        STALENESS_THRESHOLD,
        &feed_id
    )?;

    let final_price = (price.price as f64) * (10f64).powi(price.exponent as i32);

    msg!("The price is ({} ± {}) * 10^{}", price.price, price.conf, price.exponent);
    msg!("The price is: {}", final_price);

    Ok(final_price)
}

pub fn fetch_sol_price(price_account: &Account<PriceUpdateV2>) -> Result<f64> {
    msg!("Fetching SOL price...");
    let feed_id: [u8; 32] = get_feed_id_from_hex(
        "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
    )?;
    let price = price_account.get_price_no_older_than(
        &Clock::get()?,
        STALENESS_THRESHOLD,
        &feed_id
    )?;
    let final_price = (price.price as f64) * (10f64).powi(price.exponent as i32);
    msg!("SOL price: {}", final_price);
    Ok(final_price)
}

pub fn fetch_eth_price(price_account: &Account<PriceUpdateV2>) -> Result<f64> {
    msg!("Fetching ETH price...");
    let feed_id: [u8; 32] = get_feed_id_from_hex(
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace"
    )?;
    let price = price_account.get_price_no_older_than(
        &Clock::get()?,
        STALENESS_THRESHOLD,
        &feed_id
    )?;
    let final_price = (price.price as f64) * (10f64).powi(price.exponent as i32);
    msg!("ETH price: {}", final_price);
    Ok(final_price)
}

pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
    if ctx.accounts.authority.key.to_string() != ADMIN_KEY {
        return Err(ErrorCode::Unauthorized.into());
    }
    msg!("Initializing treasury account...");
    Ok(())
}

pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    strike: u64,
    expiry: i64,
    asset: u8
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.authority = ctx.accounts.authority.key();
    market.strike = strike;
    market.expiry = expiry;
    market.asset = asset;
    market.resolved = false;
    market.outcome = None;

    msg!("Market initialized!");

    msg!("Next step: call CreateOutcomeTokens ");
    Ok(())
}

#[inline(never)]
pub fn initialize_outcome_mints(ctx: Context<InitializeOutcomeMints>) -> Result<()> {
    let market = &ctx.accounts.market;
    require!(market.resolved == false, ErrorCode::MarketAlreadyResolved);

    //Normally can't mint is deterministic so they're can't be infinitely minted ?

    msg!("✅ YES and NO Mints Created!");
    Ok(())
}

//This isn't used currently
#[inline(never)]
pub fn initialize_treasury_token_accounts(
    _ctx: Context<InitializeTreasuryTokenAccounts>
) -> Result<()> {
    msg!("✅ Treasury Token Accounts Initialized!");
    Ok(())
}

#[inline(never)]
pub fn mint_outcome_tokens(ctx: Context<MintOutcomeTokens>) -> Result<()> {
    let token_program = &ctx.accounts.token_program;

    if ctx.accounts.yes_mint.mint_authority != Some(ctx.accounts.market.key()).into() {
        return Err(ErrorCode::InvalidMintAccount.into());
    }

    let market_key = &ctx.accounts.market.authority.key();

    let market_seeds = &[
        b"market",
        market_key.as_ref(),
        &ctx.accounts.market.strike.to_le_bytes(),
        &ctx.accounts.market.expiry.to_le_bytes(),
        &[ctx.bumps.market],
    ];
    let signer = &[&market_seeds[..]];

    // ✅ Mint 500,000 YES Tokens
    // let binding: [&[&[u8]];1] = [market_seeds];
    let yes_mint_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.yes_mint.to_account_info(),
            to: ctx.accounts.treasury_yes_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer
    );

    token::mint_to(yes_mint_ctx, 500_000)?;

    // ✅ Mint 500,000 NO Tokens
    let no_mint_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.no_mint.to_account_info(),
            to: ctx.accounts.treasury_no_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer
    );
    token::mint_to(no_mint_ctx, 500_000)?;

    msg!("✅ 500,000 YES and NO tokens successfully minted!");
    Ok(())
}

pub fn create_mint(
    ctx: Context<CreateMint>,
) -> Result<()> {
    
    let market_key = &ctx.accounts.market.key();
    let bump=&[ ctx.bumps.yes_mint];
    let seeds = &["yes_mint".as_bytes(), market_key.as_ref(), bump];
    let signer:&[&[&[u8]]] = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata_account.to_account_info(), // the metadata account being created
            mint: ctx.accounts.yes_mint.to_account_info(), // the mint account of the metadata account
            mint_authority: ctx.accounts.yes_mint.to_account_info(), // the mint authority of the mint account
            update_authority: ctx.accounts.yes_mint.to_account_info(), // the update authority of the metadata account
            payer: ctx.accounts.authority.to_account_info(), // the payer for creating the metadata account
            system_program: ctx.accounts.system_program.to_account_info(), // the system program account
            rent: ctx.accounts.rent.to_account_info(), // the rent sysvar account
        },
        signer
    );

    let market = &ctx.accounts.market;
    let name=  format!("{} {} {} {}", market.asset, market.strike, market.expiry, market.asset);
    let symbol = "YES".to_string();
    let uri = "Nothing for the moment".to_string();
    
    create_metadata_accounts_v3(
        cpi_ctx, // cpi context
        DataV2 {
            name: name,
            symbol: symbol,
            uri: uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        }, // token metadata
        false, // is_mutable
        false, // update_authority_is_signer
        None // collection details
    )?;

    msg!("Succesfully initialized token mint");

    Ok(())
}

pub fn mint_metadata_tokens(ctx: Context<MintMetadataTokens>) -> Result<()> {
    let token_program = &ctx.accounts.token_program;
    // let market_seeds = &[
    //     b"market",
    //     market_key.as_ref(),
    //     &ctx.accounts.market.strike.to_le_bytes(),
    //     &ctx.accounts.market.expiry.to_le_bytes(),
    //     &[ctx.bumps.market],
    // ];

    let market_key = &ctx.accounts.market.key();
    let bump=&[ ctx.bumps.yes_mint];
    let seeds = &["yes_mint".as_bytes(), market_key.as_ref(), bump];
    let signer:&[&[&[u8]]] = &[&seeds[..]];

    let yes_mint_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.yes_mint.to_account_info(),
            to: ctx.accounts.treasury_yes_token_account.to_account_info(),
            authority: ctx.accounts.yes_mint.to_account_info(),
        },
        signer
    );
    token::mint_to(yes_mint_ctx, 500_000)?;
    msg!("Minted 500 000 yes tokens!");


    Ok(())
}
