use std::num::TryFromIntError;
use std::str::FromStr;

use anchor_lang::{ prelude::*, solana_program };
// use SolanaPriceAccount::account_to_feed;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use pyth_solana_receiver_sdk::price_update::get_feed_id_from_hex;
use crate::state::*;
use crate::error::ErrorCode;
use anchor_spl::token::{
    self,
    Mint,
    Token,
    TokenAccount,
    InitializeMint,
    MintTo,
    spl_token,
    Burn,
    Transfer,
};

pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
    msg!("Resolving market...");
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

// pub fn mint_token(ctx: Context<MintToken>) -> Result<()> {
//     let mint_amount = 1_000; // Mint 1000 tokens

//     // Mint the tokens
//     let mint_to_ix = MintTo {
//         mint: ctx.accounts.mint.to_account_info(),
//         to: ctx.accounts.recipient_token_account.to_account_info(),
//         authority: ctx.accounts.authority.to_account_info(),
//     };

//     anchor_spl::token::mint_to(
//         CpiContext::new(
//             ctx.accounts.token_program.to_account_info(),
//             mint_to_ix,
//         ),
//         mint_amount,
//     )?;

//     msg!("Minted {} tokens to recipient!", mint_amount);
//     Ok(())
// }

// pub fn mint_tokens(ctx: Context<MintOutcomeTokens>, amount: u64) -> Result<()> {
//     let token_program = &ctx.accounts.token_program;

//     // Derive Mint Authority PDA
//     let (mint_authority_pda, _bump) = Pubkey::find_program_address(
//         &[b"mint_auth", ctx.accounts.market.key().as_ref()],
//         ctx.program_id,
//     );

//     let cpi_accounts_yes = MintTo {
//         mint: ctx.accounts.yes_mint.to_account_info(),
//         to: ctx.accounts.user_yes_token_account.to_account_info(),
//         authority: ctx.accounts.market.to_account_info(),
//     };

//     let cpi_accounts_no = MintTo {
//         mint: ctx.accounts.no_mint.to_account_info(),
//         to: ctx.accounts.user_no_token_account.to_account_info(),
//         authority: ctx.accounts.market.to_account_info(),
//     };

//     let cpi_context_yes = CpiContext::new(token_program.to_account_info(), cpi_accounts_yes);
//     let cpi_context_no = CpiContext::new(token_program.to_account_info(), cpi_accounts_no);

//     // Mint both YES and NO tokens
//     token::mint_to(cpi_context_yes.with_signer(&[&[b"mint_auth", ctx.accounts.market.key().as_ref(), &[_bump]]]), amount)?;
//     token::mint_to(cpi_context_no.with_signer(&[&[b"mint_auth", ctx.accounts.market.key().as_ref(), &[_bump]]]), amount)?;

//     msg!("Minted {} YES and NO tokens", amount);
//     Ok(())
// }

// pub fn create_outcome_tokens(ctx: Context<CreateOutcomeTokens>) -> Result<()> {
//     let token_program = &ctx.accounts.token_program;

//     //Assure that only the admin can create outcome tokens, temporary
//     if ctx.accounts.authority.key().to_string() != ADMIN_KEY.to_string() {
//         return Err(ErrorCode::Unauthorized.into());
//     }
//     if ctx.accounts.market.resolved==true  {
//         return Err(ErrorCode::MarketAlreadyResolved.into());
//     }
//     let current_time= Clock::get()?.unix_timestamp;
//     if ctx.accounts.market.expiry >current_time {
//         return Err(ErrorCode::MarketAlreadyExpired.into());
//     }

//     // Mint 500,000 YES Tokens to the treasury
//     let yes_mint_ctx = CpiContext::new(
//         token_program.to_account_info(),
//         MintTo {
//             mint: ctx.accounts.yes_mint.to_account_info(),
//             to: ctx.accounts.treasury_yes_token_account.to_account_info(),
//             authority: ctx.accounts.market.to_account_info(),
//         },
//     );
//     token::mint_to(yes_mint_ctx, 500_000)?;

//     // Mint 500,000 NO Tokens to the treasury
//     let no_mint_ctx = CpiContext::new(
//         token_program.to_account_info(),
//         MintTo {
//             mint: ctx.accounts.no_mint.to_account_info(),
//             to: ctx.accounts.treasury_no_token_account.to_account_info(),
//             authority: ctx.accounts.market.to_account_info(),
//         },
//     );
//     token::mint_to(no_mint_ctx, 500_000)?;

//     msg!("500,000 YES and NO tokens minted and transferred to treasury!");

//     Ok(())
// }
// #[inline(never)]
// fn mint_outcome_tokens(ctx: Context<CreateOutcomeTokens>) -> Result<()> {
//     let token_program = ctx.accounts.token_program.to_account_info();

//     // ✅ Extract accounts to ensure consistent lifetimes
//     let yes_mint = ctx.accounts.yes_mint.to_account_info();
//     let no_mint = ctx.accounts.no_mint.to_account_info();
//     let treasury_yes_token_account = ctx.accounts.treasury_yes_token_account.to_account_info();
//     let treasury_no_token_account = ctx.accounts.treasury_no_token_account.to_account_info();
//     let market = ctx.accounts.market.to_account_info();

//     // ✅ Mint 500,000 YES Tokens to the treasury
//     let yes_mint_ctx = CpiContext::new(
//         token_program.clone(),  // Use the extracted reference
//         MintTo {
//             mint: yes_mint.clone(),
//             to: treasury_yes_token_account.clone(),
//             authority: market.clone(),
//         },
//     );
//     token::mint_to(yes_mint_ctx, 500_000)?;

//     // ✅ Mint 500,000 NO Tokens to the treasury
//     let no_mint_ctx = CpiContext::new(
//         token_program,
//         MintTo {
//             mint: no_mint,
//             to: treasury_no_token_account,
//             authority: market,
//         },
//     );
//     token::mint_to(no_mint_ctx, 500_000)?;

//     msg!("✅ 500,000 YES and NO tokens minted and transferred to treasury!");

//     Ok(())
// }

// #[inline(never)]
// fn mint_tokens<'a,'b,'c,'info>(ctx: CpiContext<'a, 'b ,'c,'info, MintTo>, amount: u64) -> Result<()> {
//     token::mint_to(ctx, amount)?;
//     msg!("✅ {} tokens minted!", amount);
//     Ok(())
// }

// #[inline(never)]
// fn check_market_conditions(ctx: &Context<CreateOutcomeTokens>) -> Result<()> {
//     // Ensure only admin can create outcome tokens
//     if ctx.accounts.authority.key().to_string() != ADMIN_KEY.to_string() {
//         return Err(ErrorCode::Unauthorized.into());
//     }
//     if ctx.accounts.market.resolved {
//         return Err(ErrorCode::MarketAlreadyResolved.into());
//     }
//     let current_time = Clock::get()?.unix_timestamp;
//     if ctx.accounts.market.expiry < current_time {
//         return Err(ErrorCode::MarketAlreadyExpired.into());
//     }
//     Ok(())
// }

// #[inline(never)]
// pub fn create_outcome_tokens<'info>(ctx: Context<'_, '_, '_, 'info, CreateOutcomeTokens<'info>>) -> Result<()> {
//     // ✅ Validate market conditions separately
//     check_market_conditions(&ctx)?;

//     let (yes_mint_pda, bump) = Pubkey::find_program_address(
//         &[b"yes_mint", ctx.accounts.token_program.key().as_ref(), ctx.accounts.market.key().as_ref()],
//         *&ctx.accounts.associated_token_program.key,
//     );

//     msg!("yes_mint_pda: {}, bump: {}", yes_mint_pda.to_string(),bump );

//     // ✅ Get accounts dynamically from `remaining_accounts`
//     let remaining_accounts = &ctx.remaining_accounts;
//     let yes_mint = remaining_accounts.get(0).ok_or(ErrorCode::InvalidMintAccount)?.to_account_info().clone();
//     let no_mint = remaining_accounts.get(1).ok_or(ErrorCode::InvalidMintAccount)?.to_account_info().clone();
//     let treasury_yes_token_account = remaining_accounts.get(2).ok_or(ErrorCode::InvalidTreasuryTokenAccount)?.to_account_info().clone();
//     let treasury_no_token_account = remaining_accounts.get(3).ok_or(ErrorCode::InvalidTreasuryTokenAccount)?.to_account_info().clone();

//     let token_program = ctx.accounts.token_program.to_account_info().clone();
//     let market = ctx.accounts.market.to_account_info().clone();

//     // ✅ Mint YES Tokens
//     let yes_mint_ctx = CpiContext::new(
//         token_program.clone(),
//         MintTo {
//             mint: yes_mint.clone(),
//             to: treasury_yes_token_account.clone(),
//             authority: market.clone(),
//         },
//     );
//     token::mint_to(yes_mint_ctx, 500_000)?;

//     // ✅ Mint NO Tokens
//     let no_mint_ctx = CpiContext::new(
//         token_program,
//         MintTo {
//             mint: no_mint.clone(),
//             to: treasury_no_token_account.clone(),
//             authority: market.clone(),
//         },
//     );
//     token::mint_to(no_mint_ctx, 500_000)?;
//     // mint_tokens(no_mint_ctx, 500_000)?;

//     msg!("✅ 500,000 YES and NO tokens successfully minted to treasury!");

//     Ok(())
// }

pub fn lock_funds(ctx: Context<LockFunds>, amount: u64) -> Result<()> {
    const LOCK_AMOUNT_PER_TOKEN: u64 = 100_000;
    let payment = amount * LOCK_AMOUNT_PER_TOKEN;
    let authority = &ctx.accounts.authority;
    // ✅ Transfer lamports from user to treasury (locking funds)
    anchor_lang::solana_program::program::invoke(
        &anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.treasury.key(),
            payment
        ),
        &[
            ctx.accounts.user.to_account_info(),
            ctx.accounts.treasury.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ]
    )?;

    msg!("{} lamports locked in the treasury!", payment);

    // ✅ Transfer 1 YES token from Treasury to the User
    let yes_transfer = Transfer {
        from: ctx.accounts.treasury_yes_token_account.to_account_info(),
        to: ctx.accounts.user_yes_token_account.to_account_info(),
        authority: ctx.accounts.treasury.to_account_info(),
    };
    let bump = ctx.bumps.treasury;
    let binding = authority.key();
    let treasury_seeds: &[&[u8]] = &[b"treasury".as_ref(), &binding.as_ref(), &[bump]];
    let signature_seeds = [treasury_seeds];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        yes_transfer,
        &signature_seeds
    );
    token::transfer(cpi_ctx, amount)?;

    msg!("{} YES token transferred to user!", amount);

    // ✅ Transfer 1 NO token from Treasury to the User
    let no_transfer = Transfer {
        from: ctx.accounts.treasury_no_token_account.to_account_info(),
        to: ctx.accounts.user_no_token_account.to_account_info(),
        authority: ctx.accounts.treasury.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        no_transfer,
        &signature_seeds
    );
    token::transfer(cpi_ctx, amount)?;

    msg!("{} NO token transferred to user!", amount);

    Ok(())
}

pub fn redeem(ctx: Context<Redeem>, amount: u64) -> Result<()> {
    let market = &ctx.accounts.market;
    let user = &ctx.accounts.user;

    require!(market.resolved, ErrorCode::MarketNotResolved);

    let (token_to_burn, mint_to_burn) = match market.outcome {
        Some(1) => (&ctx.accounts.user_yes_token_account, &ctx.accounts.yes_mint),
        Some(2) => (&ctx.accounts.user_no_token_account, &ctx.accounts.no_mint),
        _ => {
            return Err(ErrorCode::InvalidMarketOutcome.into());
        }
    };

    require!(token_to_burn.mint == mint_to_burn.key(), ErrorCode::InvalidTokenMint);

    // Burn user's tokens
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), Burn {
        mint: mint_to_burn.to_account_info(),
        from: token_to_burn.to_account_info(),
        authority: user.to_account_info(),
    });
    token::burn(cpi_ctx, amount)?;

    let lamports_to_transfer = amount * 100_000;

    **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= lamports_to_transfer;
    **user.to_account_info().try_borrow_mut_lamports()? += lamports_to_transfer;

    msg!("Redeemed {} tokens, transferred {} lamports to user", amount, lamports_to_transfer);
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



#[inline(never)]
pub fn initialize_outcome_mints(ctx: Context<InitializeOutcomeMints>) -> Result<()> {
    let market = &ctx.accounts.market;

    require!(market.resolved == false, ErrorCode::MarketAlreadyResolved);

    msg!("✅ YES and NO Mints Created!");
    Ok(())
}

#[inline(never)]
pub fn initialize_treasury_token_accounts(ctx: Context<InitializeTreasuryTokenAccounts>) -> Result<()> {
    msg!("✅ Treasury Token Accounts Initialized!");
    Ok(())
}
#[inline(never)]
pub fn mint_outcome_tokens(ctx: Context<MintOutcomeTokens>) -> Result<()> {
    let token_program = &ctx.accounts.token_program;

    // ✅ Mint 500,000 YES Tokens
    let yes_mint_ctx = CpiContext::new(
        token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.yes_mint.to_account_info(),
            to: ctx.accounts.treasury_yes_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
    );
    token::mint_to(yes_mint_ctx, 500_000)?;

    // ✅ Mint 500,000 NO Tokens
    let no_mint_ctx = CpiContext::new(
        token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.no_mint.to_account_info(),
            to: ctx.accounts.treasury_no_token_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
    );
    token::mint_to(no_mint_ctx, 500_000)?;

    msg!("✅ 500,000 YES and NO tokens successfully minted!");
    Ok(())
}