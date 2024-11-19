use crate::canopy::fill_in_proof_from_canopy;
use crate::compressed_nfts::{verify_compressed_nft, VerifyCompressedNftArgs};
use crate::error::RdError;
use crate::{id, merkle_proof::verify as verify_distribution_tree, state::*};
use account_compression_cpi::program::SplAccountCompression;
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use bubblegum_cpi::get_asset_id;
use circuit_breaker::cpi::accounts::TransferV0;
use circuit_breaker::cpi::transfer_v0;
use circuit_breaker::{AccountWindowedCircuitBreakerV0, CircuitBreaker, TransferArgsV0};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct CompressedNftVerification {
    pub root: [u8; 32],
    pub index: u32,
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
    pub proof_size: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct DistributionTreeVerification {
    pub index: u32,
    pub data_hash: [u8; 32],
    pub current_period_rewards: u64,
    pub total_rewards: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ClaimRewardsArgs {
    pub compressed_nft_args: CompressedNftVerification,
    pub distribution_tree_args: DistributionTreeVerification,
}

#[derive(Accounts)]
#[instruction(args: ClaimRewardsArgs)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
      has_one = rewards_mint,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    #[account(
      has_one = distributor,
      has_one = canopy,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    /// CHECK: Verified by has one
    pub canopy: UncheckedAccount<'info>,

    #[account(
      init_if_needed,
      payer = payer,
      space = 8 + std::mem::size_of::<Recipient>(),
      seeds = [
        "recipient".as_bytes(), 
        distributor.key().as_ref(),
        get_asset_id(&merkle_tree.key(), args.compressed_nft_args.index.into()).as_ref()
      ],
      bump,
    )]
    pub recipient: Box<Account<'info, Recipient>>,

    pub rewards_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::authority = distributor,
        associated_token::mint = rewards_mint,
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
      mut,
      seeds = ["account_windowed_breaker".as_bytes(), vault.key().as_ref()],
      seeds::program = circuit_breaker_program.key(),
      bump = circuit_breaker.bump_seed
    )]
    pub circuit_breaker: Box<Account<'info, AccountWindowedCircuitBreakerV0>>,

    /// CHECK: see destination_account
    pub owner: AccountInfo<'info>,

    #[account(
      init_if_needed,
      payer = payer,
      associated_token::mint = rewards_mint,
      associated_token::authority = owner,
    )]
    pub destination_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: THe merkle tree
    pub merkle_tree: UncheckedAccount<'info>,
    pub compression_program: Program<'info, SplAccountCompression>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub circuit_breaker_program: Program<'info, CircuitBreaker>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn claim_rewards<'info>(
    ctx: Context<'_, '_, 'info, 'info, ClaimRewards<'info>>,
    args: ClaimRewardsArgs,
) -> Result<()> {
    let distributor = &ctx.accounts.distributor;
    let distribution_tree = &ctx.accounts.distribution_tree;
    let recipient = &mut ctx.accounts.recipient;
    let asset = get_asset_id(
        &ctx.accounts.merkle_tree.key(),
        args.compressed_nft_args.index.into(),
    );

    // make sure the distribution tree is active.
    require_gte!(
        distributor.current_period,
        distribution_tree.period,
        RdError::DistributionTreeIsInactive
    );

    // initialize recipient struct if needed
    if recipient.distributor == Pubkey::default() {
        recipient.distributor = ctx.accounts.distributor.key();
        recipient.asset = asset;
        recipient.claimed_rewards = 0;
        recipient.last_claim_period = 0;
        recipient.bump = ctx.bumps.recipient;
    }

    // verify recipient
    require_eq!(ctx.accounts.distributor.key(), recipient.distributor, RdError::InvalidRecipient);
    require_eq!(recipient.asset, asset, RdError::InvalidAsset);
    require_gt!(distribution_tree.period, recipient.last_claim_period, RdError::ExpiredPeriod);

    // Verify the compressed nft to make sure the owner is correct.
    let proof_accounts = ctx.remaining_accounts;
    verify_compressed_nft(VerifyCompressedNftArgs {
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        compression_program: ctx.accounts.compression_program.to_account_info(),
        data_hash: args.compressed_nft_args.data_hash,
        creator_hash: args.compressed_nft_args.creator_hash,
        owner: ctx.accounts.owner.key(),
        delegate: ctx.accounts.owner.key(),
        root: args.compressed_nft_args.root,
        index: args.compressed_nft_args.index,
        proof_accounts: proof_accounts[0..args.compressed_nft_args.proof_size as usize].to_vec(),
    })?;

    // verify distribution tree to make sure the total_rewards is correct.
    let mut distribution_tree_proof = proof_accounts
        .iter()
        .skip(args.compressed_nft_args.proof_size as usize)
        .map(|a| a.key.to_bytes())
        .collect::<Vec<_>>();

    // fill proof only if canopy account is valid
    if *ctx.accounts.canopy.owner == id() {
        fill_in_proof_from_canopy(
            &ctx.accounts.canopy.try_borrow_data()?[1..],
            distribution_tree.max_depth,
            args.distribution_tree_args.index,
            &mut distribution_tree_proof,
        )?;
    }

    let dist_tree_leaf_hash = solana_program::keccak::hashv(&[
        asset.as_ref(),
        &args.distribution_tree_args.data_hash[..],
        &args.distribution_tree_args.current_period_rewards.to_be_bytes(),
        &args.distribution_tree_args.total_rewards.to_be_bytes(),
    ]).0;
    if !verify_distribution_tree(
        distribution_tree_proof,
        distribution_tree.root,
        dist_tree_leaf_hash,
        args.distribution_tree_args.index,
    ) {
        return Err(error!(RdError::InvalidProof));
    };

    // calculate amount to dist, then transfer funds from vault to destination account.
    let security_reward_limit = (distribution_tree.period as u64)
        .checked_sub(recipient.last_claim_period as u64)
        .unwrap()
        .checked_mul(distributor.security_rewards_limit)
        .unwrap();

    let amount_to_dist = args.distribution_tree_args
        .total_rewards
        .checked_sub(recipient.claimed_rewards)
        .unwrap();
    require_gte!(security_reward_limit, amount_to_dist, RdError::SecurityControl);

    let distributor_seeds: &[&[&[u8]]] = &[&[
        b"distributor",
        distributor.realm.as_ref(),
        distributor.rewards_mint.as_ref(),
        &[distributor.bump],
    ]];
    transfer_v0(
        CpiContext::new_with_signer(
            ctx.accounts
                .circuit_breaker_program
                .to_account_info()
                .clone(),
            TransferV0 {
                from: ctx.accounts.vault.to_account_info().clone(),
                to: ctx.accounts.destination_account.to_account_info().clone(),
                owner: ctx.accounts.distributor.to_account_info().clone(),
                circuit_breaker: ctx.accounts.circuit_breaker.to_account_info().clone(),
                token_program: ctx.accounts.token_program.to_account_info().clone(),
            },
            distributor_seeds,
        ),
        TransferArgsV0 {
            amount: amount_to_dist,
        },
    )?;

    // update recipient
    recipient.claimed_rewards = args.distribution_tree_args.total_rewards;
    recipient.last_claim_period = distribution_tree.period;

    Ok(())
}
