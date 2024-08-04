use anchor_lang::prelude::*;

declare_id!("CirC9HGGQgTk8XA8ARBgXkaBDDt3Jejs3F8ezTNKp8Q");

pub mod errors;
pub mod instructions;
pub mod state;
pub mod window;

pub use instructions::*;
pub use state::*;

#[derive(Clone)]
pub struct CircuitBreaker;

impl anchor_lang::Id for CircuitBreaker {
  fn id() -> Pubkey {
    crate::id()
  }
}

#[program]
pub mod circuit_breaker {
  use super::*;

  pub fn initialize_mint_windowed_breaker_v0(
    ctx: Context<InitializeMintWindowedBreakerV0>,
    args: InitializeMintWindowedBreakerArgsV0,
  ) -> Result<()> {
    instructions::initialize_mint_windowed_breaker_v0::initialize_mint_windowed_breaker(ctx, args)
  }

  pub fn initialize_account_windowed_breaker_v0(
    ctx: Context<InitializeAccountWindowedBreakerV0>,
    args: InitializeAccountWindowedBreakerArgsV0,
  ) -> Result<()> {
    instructions::initialize_account_windowed_breaker_v0::initialize_account_windowed_breaker(ctx, args)
  }

  pub fn mint_v0(ctx: Context<MintV0>, args: MintArgsV0) -> Result<()> {
    instructions::mint_v0::mint(ctx, args)
  }

  pub fn transfer_v0(ctx: Context<TransferV0>, args: TransferArgsV0) -> Result<()> {
    instructions::transfer_v0::transfer(ctx, args)
  }

  pub fn update_account_windowed_breaker_v0(
    ctx: Context<UpdateAccountWindowedBreakerV0>,
    args: UpdateAccountWindowedBreakerArgsV0,
  ) -> Result<()> {
    instructions::update_account_windowed_breaker_v0::update_account_windowed_breaker(ctx, args)
  }

  pub fn update_mint_windowed_breaker_v0(
    ctx: Context<UpdateMintWindowedBreakerV0>,
    args: UpdateMintWindowedBreakerArgsV0,
  ) -> Result<()> {
    instructions::update_mint_windowed_breaker_v0::update_mint_windowed_breaker(ctx, args)
  }
}
