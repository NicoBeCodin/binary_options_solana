use anchor_lang::{prelude::*, accounts::account::Account};
use crate::state::*;



pub mod instructions;
pub mod state;
pub mod error;

declare_id!("8wpsKAtskF9J7fo6KEEjPYacHS7JCXgzZnM1E1Un53tN");



#[program]
pub mod binary_options {
    use super::*;

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        strike: u64,
        expiry: i64,
        asset: u8,
    ) -> Result<()> {
        instructions::initialize_market(ctx, strike, expiry, asset)
    }

    pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
        instructions::resolve_market(ctx)
    }
    pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
        instructions::initialize_treasury(ctx)
    }
    pub fn create_outcome_tokens(ctx: Context<CreateOutcomeTokens>) -> Result<()>{
        instructions::create_outcome_tokens(ctx)
    }

    pub fn get_price_feed(ctx: Context<GetPriceFeed>, feed_id_str: String) -> Result<f64> {
        instructions::get_price_feed(ctx, feed_id_str)
    }

    pub fn fetch_coin_price(ctx: Context<FetchCoinPrice>, coin: i8) -> Result<f64> {
        
        match coin {   
        1=>  return instructions::fetch_btc_price(&ctx.accounts.price_update),
        2 =>  return instructions::fetch_sol_price(&ctx.accounts.price_update),
        3 => return instructions::fetch_eth_price(&ctx.accounts.price_update),
        _ => return Err(error::ErrorCode::InvalidCoin.into())
            
        }
    }


    pub fn fetch_btc_price(ctx: Context<FetchCoinPrice>) -> Result<f64> {
        instructions::fetch_btc_price(&ctx.accounts.price_update)
    }
    pub fn redeem(ctx: Context<Redeem>, amount: u64)->Result<()>{
        instructions::redeem(ctx, amount)
    }

}










