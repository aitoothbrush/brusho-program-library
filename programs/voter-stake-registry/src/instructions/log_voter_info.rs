use crate::events::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct LogVoterInfo<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(
        constraint = voter.load()?.get_registrar() == registrar.key()
    )]
    pub voter: AccountLoader<'info, Voter>,
}

/// A no-effect instruction that logs information about the voter and deposits.
pub fn log_voter_info(ctx: Context<LogVoterInfo>) -> Result<()> {
    let registrar = &ctx.accounts.registrar.load()?;
    let voter = &ctx.accounts.voter.load()?;

    let curr_ts = registrar.clock_unix_timestamp();
    let mut deposit_entries: [Option<DepositEntryInfo>; VOTER_DEPOSIT_ENTRY_COUNT] = Default::default();
    for (index, d_entry) in voter.get_deposits().iter().enumerate() {
        if d_entry.is_active() {
            let lockup = &d_entry.get_lockup();
            let periods_total = lockup.periods_total();
            let periods_left = lockup.periods_left(curr_ts)?;
            let amount_locked = d_entry.amount_locked(curr_ts)?;
            let amount_unlocked = d_entry.amount_unlocked(curr_ts)?;
            let voting_power = d_entry.voting_power(&registrar.voting_config, curr_ts)?;
            let voting_power_baseline = registrar
                .voting_config
                .baseline_vote_weight(d_entry.get_amount_deposited_native())?;
            let vesting = lockup.kind.is_vesting().then(|| VestingInfo {
                rate: d_entry
                    .get_amount_initially_locked_native()
                    .checked_div(periods_total)
                    .unwrap(),
                next_timestamp: (d_entry.get_lockup().end_ts() as u64).saturating_sub(
                    periods_left
                        .saturating_sub(1)
                        .checked_mul(lockup.kind.period_secs())
                        .unwrap(),
                ),
            });

            deposit_entries[index] = Some(DepositEntryInfo {
                lockup: d_entry.get_lockup(),
                amount_locked,
                amount_unlocked,
                voting_power,
                voting_power_baseline,
                vesting,
            });
        }
    }

    let seconds_delta = curr_ts.checked_sub(registrar.reward_accrual_ts).unwrap() as u64;
    let reward_index_delta = if registrar.permanently_locked_amount != 0 {
        registrar
            .current_reward_amount_per_second
            .mul_scalar(seconds_delta as core::primitive::u128)
            .div_scalar(u64::max(
                registrar.permanently_locked_amount,
                FULL_REWARD_PERMANENTLY_LOCKED_FLOOR,
            ) as core::primitive::u128)
    } else {
        u128::new(0)
    };

    let reward_amount = voter
        .get_reward_claimable_amount()
        .checked_add(
            registrar
                .reward_index
                .add(reward_index_delta)
                .sub(voter.get_reward_index())
                .mul_scalar(voter.permanently_locked(curr_ts)? as core::primitive::u128)
                .truncate() as u64,
        )
        .unwrap();

    emit!(VoterInfo {
        voting_power: voter.weight(curr_ts, registrar)?,
        voting_power_baseline: voter.weight_baseline(registrar)?,
        reward_amount,
        deposit_entries
    });

    Ok(())
}
