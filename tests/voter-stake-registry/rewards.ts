import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, createRealm, newSigner, VSR_PROGRAM, lockupDayily, LockupTimeDuration, newTokenAccount, mintTokenToAccount, createRegistrar, createVoter, defaultDepositConfig, defaultVotingConfig, CONNECTION, CIRCUIT_BREAKER_PROGRAM, fastup, SECS_PER_DAY, assertThrowsSendTransactionError, getTokenAccount, EXP_SCALE, FULL_REWARD_PERMANENTLY_LOCKED_FLOOR, SECS_PER_YEAR, TOTAL_REWARD_AMOUNT } from "./helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


describe("rewards!", () => {
    const depositAmount = new anchor.BN(1e9); // 1000 tokens
    const circuitBreakerThreshold = new anchor.BN(1e7); // 10 tokens

    let authority: web3.Keypair;
    let mint: web3.PublicKey;
    let councilMint: web3.PublicKey;
    let realm: web3.PublicKey;
    let registrar: web3.PublicKey;
    let registrarBump: number;
    let registrarVault: web3.PublicKey;
    let circuitBreaker: web3.PublicKey;
    let voterAuthority: web3.Keypair;
    let voter: web3.PublicKey;
    let voterWeightRecord: web3.PublicKey;
    let vault: web3.PublicKey;
    let tokenOwnerRecord: web3.PublicKey;
    let depositToken: web3.PublicKey;

    before(async () => {
        authority = await newSigner();
        [mint, councilMint, realm] = await createRealm(authority);
        // create registrar
        [registrar, registrarBump, registrarVault, circuitBreaker] = await createRegistrar(realm, authority, mint, defaultVotingConfig(), defaultDepositConfig(), circuitBreakerThreshold, authority);
        [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
        depositToken = await newTokenAccount(mint, voterAuthority);

        // console.log(`authority: ${authority.publicKey.toBase58()}`)
        // console.log(`mint: ${mint.toBase58()}`)
        // console.log(`councilMint: ${councilMint.toBase58()}`)
        // console.log(`realm: ${realm.toBase58()}`)
        // console.log(`registrar: ${registrar.toBase58()}`)
        // console.log(`registrarVault: ${registrarVault.toBase58()}`)
        // console.log(`circuitBreaker: ${circuitBreaker.toBase58()}`)
        // console.log(`voterAuthority: ${voterAuthority.publicKey.toBase58()}`)
        // console.log(`voter: ${voter.toBase58()}`)
        // console.log(`voterWeightRecord: ${voterWeightRecord.toBase58()}`)
        // console.log(`vault: ${vault.toBase58()}`)
        // console.log(`tokenOwnerRecord: ${tokenOwnerRecord.toBase58()}`)
        // console.log(`depositToken: ${depositToken.toBase58()}`)
    })

    async function deposit(depositEntryIndex: number, duration: LockupTimeDuration): Promise<string> {
        await mintTokenToAccount(mint, authority, depositToken, depositAmount)
        return await VSR_PROGRAM.methods
            .ordinaryDeposit(depositEntryIndex, depositAmount, duration)
            .accounts({
                registrar,
                voter,
                vault,
                depositToken,
                depositAuthority: voterAuthority.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
            }).signers([voterAuthority])
            .rpc({ commitment: "confirmed" });
    }

    it("with_incorrect_registrar_should_fail", async () => {
        let [invalidRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);

        await assertThrowsAnchorError('ConstraintSeeds', async () => {
            await VSR_PROGRAM.methods
                .claimReward(null)
                .accounts({
                    registrar: invalidRegistrar,
                    voter,
                    voterAuthority: voterAuthority.publicKey,
                    destination: depositToken,
                    vault: registrarVault,
                    circuitBreaker,
                    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
                }).signers([voterAuthority])
                .rpc();
        },
            (anchorErr) => {
                if (anchorErr.error.comparedValues) {
                    assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
                }
            },
            false
        );
    });

    it("with_incorrect_voter_authority_should_fail", async () => {
        const invalidVoterAuthority = await newSigner();

        await assertThrowsAnchorError('ConstraintSeeds', async () => {
            await VSR_PROGRAM.methods
                .claimReward(null)
                .accounts({
                    registrar: registrar,
                    voter,
                    voterAuthority: invalidVoterAuthority.publicKey,
                    destination: depositToken,
                    vault: registrarVault,
                    circuitBreaker,
                    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
                }).signers([invalidVoterAuthority])
                .rpc();
        },
            (anchorErr) => {
                if (anchorErr.error.comparedValues) {
                    assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
                }
            },
            false
        );
    });

    it("with_insufficient_funds_in_vault_should_fail", async () => {
        const depositEntryIndex = 3;
        await deposit(depositEntryIndex, lockupDayily(15));

        let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");

        // fastup 1 day
        await fastup(registrar, authority, SECS_PER_DAY, "confirmed");

        // 0x1 represents TokenError::InsufficientFunds
        await assertThrowsSendTransactionError('custom program error: 0x1', async () => {
            await VSR_PROGRAM.methods
                .claimReward(null)
                .accounts({
                    registrar,
                    voter,
                    voterAuthority: voterAuthority.publicKey,
                    destination: depositToken,
                    vault: registrarVault,
                    circuitBreaker,
                    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
                }).signers([voterAuthority])
                .rpc({ commitment: "confirmed" });
        },
            (anchorErr) => { },
            false
        );
    });

    it("verify_claim_reward", async () => {
        // deposit tokens to vault
        await mintTokenToAccount(mint, authority, registrarVault, new anchor.BN(1e10));

        const destinationTokenAccount = await newTokenAccount(mint, await newSigner());
        let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        let prevRewardAccrualTs = registrarData.rewardAccrualTs;
        let prevRewardIndex = registrarData.rewardIndex;

        // fastup 1 day
        await fastup(registrar, authority, SECS_PER_DAY, "confirmed");

        // estimate rewards
        const logVoterInfoResp = await VSR_PROGRAM.methods
            .logVoterInfo()
            .accounts({
                registrar,
                voter,
            })
            .signers([])
            .simulate()

        const voterInfoData = logVoterInfoResp.events[0].data;
        const estimatedRewards = voterInfoData.rewardAmount as anchor.BN;
        const claimAmount = estimatedRewards.subn(1);

        let txId = await VSR_PROGRAM.methods
            .claimReward(claimAmount)
            .accounts({
                registrar,
                voter,
                voterAuthority: voterAuthority.publicKey,
                destination: destinationTokenAccount,
                vault: registrarVault,
                circuitBreaker,
                circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
            }).signers([voterAuthority])
            .rpc({ commitment: "confirmed" });

        let voterData = await VSR_PROGRAM.account.voter.fetch(voter, "confirmed");
        assert.isTrue(voterData.rewardClaimableAmount.gten(1));

        // verify registrar data
        registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        let tx = await CONNECTION.getTransaction(txId, { commitment: 'confirmed' });
        let txTime = registrarData.timeOffset.add(new anchor.BN(tx!.blockTime!.toString()));
        assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
        assert.equal(registrarData.rewardIndex.toString(), voterData.rewardIndex.toString())

        let rewardIndexDelta = registrarData.currentRewardAmountPerSecond.mul(registrarData.rewardAccrualTs.sub(prevRewardAccrualTs))
            .div(anchor.BN.max(registrarData.permanentlyLockedAmount, FULL_REWARD_PERMANENTLY_LOCKED_FLOOR))
        assert.equal(registrarData.rewardIndex.toString(), prevRewardIndex.add(rewardIndexDelta).toString());

        let destinationTokenAccountData = await getTokenAccount(destinationTokenAccount);
        assert.equal(claimAmount.toString(), destinationTokenAccountData.amount.toString())

        // claim remains
        await VSR_PROGRAM.methods
            .claimReward(null)
            .accounts({
                registrar,
                voter,
                voterAuthority: voterAuthority.publicKey,
                destination: destinationTokenAccount,
                vault: registrarVault,
                circuitBreaker,
                circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
            }).signers([voterAuthority])
            .rpc({ commitment: "confirmed" });

        voterData = await VSR_PROGRAM.account.voter.fetch(voter, "confirmed");
        assert.isTrue(voterData.rewardClaimableAmount.eqn(0));

        destinationTokenAccountData = await getTokenAccount(destinationTokenAccount);
        assert.isTrue(new anchor.BN(destinationTokenAccountData.amount.toString()).gte(estimatedRewards));
    });

    it("claim_reward_later_again_should_work", async () => {
        let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        let prevRewardAccrualTs = registrarData.rewardAccrualTs;
        let prevRewardIndex = registrarData.rewardIndex;

        // fastup 1 day
        await fastup(registrar, authority, SECS_PER_DAY, "confirmed");

        const destinationTokenAccount = await newTokenAccount(mint, await newSigner());

        let txId = await VSR_PROGRAM.methods
            .claimReward(null)
            .accounts({
                registrar,
                voter,
                voterAuthority: voterAuthority.publicKey,
                destination: destinationTokenAccount,
                vault: registrarVault,
                circuitBreaker,
                circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
            }).signers([voterAuthority])
            .rpc({ commitment: "confirmed" });

        let voterData = await VSR_PROGRAM.account.voter.fetch(voter, "confirmed");
        assert.isTrue(voterData.rewardClaimableAmount.eqn(0));

        // verify registrar data
        registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        let tx = await CONNECTION.getTransaction(txId, { commitment: 'confirmed' });
        let txTime = registrarData.timeOffset.add(new anchor.BN(tx!.blockTime!.toString()));
        assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
        assert.equal(registrarData.rewardIndex.toString(), voterData.rewardIndex.toString())

        let rewardIndexDelta = registrarData.currentRewardAmountPerSecond.mul(registrarData.rewardAccrualTs.sub(prevRewardAccrualTs))
            .div(anchor.BN.max(registrarData.permanentlyLockedAmount, FULL_REWARD_PERMANENTLY_LOCKED_FLOOR))
        assert.equal(registrarData.rewardIndex.toString(), prevRewardIndex.add(rewardIndexDelta).toString());

        let destinationTokenAccountData = await getTokenAccount(destinationTokenAccount);
        let expectRewardAmount = rewardIndexDelta.mul(depositAmount).div(EXP_SCALE);
        assert.equal(expectRewardAmount.toString(), destinationTokenAccountData.amount.toString())
    });

    it("create_new_voter", async () => {
        let [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);

        let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        let voterData = await VSR_PROGRAM.account.voter.fetch(voter, "confirmed");
        assert.equal(voterData.rewardIndex.toString(), registrarData.rewardIndex.toString())
    });

    it("exceeds_circuit_breaker_threshold", async () => {
        // fastup half year
        await fastup(registrar, authority, SECS_PER_YEAR.divn(2), "confirmed");

        // Trigger cumulative reward action
        const depositEntryIndex = 3;
        await deposit(depositEntryIndex, lockupDayily(15));

        const voterData = await VSR_PROGRAM.account.voter.fetch(voter, "confirmed");
        assert.isTrue(voterData.rewardClaimableAmount.gt(circuitBreakerThreshold));

        const destinationTokenAccount = await newTokenAccount(mint, await newSigner());

        await assertThrowsAnchorError('CircuitBreakerTriggered', async () => {
            await VSR_PROGRAM.methods
                .claimReward(null)
                .accounts({
                    registrar,
                    voter,
                    voterAuthority: voterAuthority.publicKey,
                    destination: destinationTokenAccount,
                    vault: registrarVault,
                    circuitBreaker,
                    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId
                }).signers([voterAuthority])
                .rpc({ commitment: "confirmed" });
        },
            undefined,
            false
        );
    });

    it("rotate_reward_amount_per_second", async () => {
        // fastup half year
        await fastup(registrar, authority, SECS_PER_YEAR.divn(2), "confirmed");

        // Trigger rotation
        const depositEntryIndex = 3;
        let txId = await deposit(depositEntryIndex, lockupDayily(15));
        let tx = await CONNECTION.getTransaction(txId, { commitment: 'confirmed' });
        let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");

        let txTime = registrarData.timeOffset.add(new anchor.BN(tx!.blockTime!.toString()));

        let expectCurrentRewardAmountPerSecond = TOTAL_REWARD_AMOUNT.sub(registrarData.issuedRewardAmount).muln(12).divn(100).mul(EXP_SCALE).div(SECS_PER_YEAR);
        assert.equal(registrarData.lastRewardAmountPerSecondRotatedTs.toString(), txTime.toString())
        assert.equal(registrarData.currentRewardAmountPerSecond.toString(), expectCurrentRewardAmountPerSecond.toString())

        // fastup half year
        await fastup(registrar, authority, SECS_PER_YEAR.divn(2), "confirmed");

        // Trigger rotation
        await deposit(depositEntryIndex, lockupDayily(15));
        registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        assert.equal(registrarData.lastRewardAmountPerSecondRotatedTs.toString(), txTime.toString()) // not changed
        assert.equal(registrarData.currentRewardAmountPerSecond.toString(), expectCurrentRewardAmountPerSecond.toString()) // not changed

        // fastup half year
        await fastup(registrar, authority, SECS_PER_YEAR.divn(2), "confirmed");

        // Trigger rotation
        txId = await deposit(depositEntryIndex, lockupDayily(15));
        registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar, "confirmed");
        tx = await CONNECTION.getTransaction(txId, { commitment: 'confirmed' });
        txTime = registrarData.timeOffset.add(new anchor.BN(tx!.blockTime!.toString()));

        expectCurrentRewardAmountPerSecond = TOTAL_REWARD_AMOUNT.sub(registrarData.issuedRewardAmount).muln(12).divn(100).mul(EXP_SCALE).div(SECS_PER_YEAR);
        assert.equal(registrarData.lastRewardAmountPerSecondRotatedTs.toString(), txTime.toString()) // changed
        assert.equal(registrarData.currentRewardAmountPerSecond.toString(), expectCurrentRewardAmountPerSecond.toString()) // changed
    });
});
