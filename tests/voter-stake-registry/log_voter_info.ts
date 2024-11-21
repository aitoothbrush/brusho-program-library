import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { createRealm, createRegistrar, createVoter, defaultDepositConfig, defaultVotingConfig, fastup, lockupDayily, lockupMonthly, LockupTimeDuration, lockupTimeDurationSeconds, mintTokenToAccount, newSigner, newTokenAccount, VSR_PROGRAM } from "../helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


describe("log_voter_info!", () => {
  let authority: web3.Keypair;
  let mint: web3.PublicKey;
  let councilMint: web3.PublicKey;
  let realm: web3.PublicKey;
  let registrar: web3.PublicKey;
  let voterAuthority: web3.Keypair;
  let voter: web3.PublicKey;
  let voterWeightRecord: web3.PublicKey;
  let vault: web3.PublicKey;
  let tokenOwnerRecord: web3.PublicKey;
  let voterTokenAccount: web3.PublicKey;

  before(async () => {
    authority = await newSigner();
    [mint, councilMint, realm] = await createRealm(authority);
    // create registrar
    [registrar] = await createRegistrar(realm, authority, mint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);
    [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
    voterTokenAccount = await newTokenAccount(mint, voterAuthority);

    // node deposit
    await mintTokenToAccount(mint, authority, voterTokenAccount, defaultDepositConfig().nodeSecurityDeposit)
    await VSR_PROGRAM.methods
      .nodeDeposit()
      .accounts({
        registrar,
        voter,
        vault: vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();
  })

  async function ordinaryDeposit(depositEntryIndex: number, duration: LockupTimeDuration, amount: anchor.BN) {
    await mintTokenToAccount(mint, authority, voterTokenAccount, amount)
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, amount, duration)
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();
  }

  it("log_voter_info", async () => {
    let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const prevPermanentlyLockedAmount = registrarData.permanentlyLockedAmount;

    // deposit at index 1
    await ordinaryDeposit(1, lockupDayily(30), new anchor.BN(1e9));
    // release deposit at index 1
    await VSR_PROGRAM.methods
      .ordinaryReleaseDeposit(1, 2, new anchor.BN(1e8))
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
      }).signers([voterAuthority])
      .rpc();

    // fastup 10 days
    await fastup(registrar, authority, lockupTimeDurationSeconds(lockupDayily(10)), "confirmed");

    // withdraw at index 2 
    await VSR_PROGRAM.methods
      .withdraw(2, new anchor.BN(1e7))
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
        tokenOwnerRecord,
        voterWeightRecord,
        vault,
        destination: voterTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID
      })
      .signers([voterAuthority])
      .rpc()

    // deposit at index 3
    await ordinaryDeposit(3, lockupMonthly(3), new anchor.BN(1e10));

    // release deposit at index 4
    await VSR_PROGRAM.methods
      .ordinaryReleaseDeposit(3, 4, new anchor.BN(1e9))
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
      }).signers([voterAuthority])
      .rpc();

    // fastup 15 days
    await fastup(registrar, authority, lockupTimeDurationSeconds(lockupDayily(15)), "confirmed");

    // call log_voter_info
    const response = await VSR_PROGRAM.methods
      .logVoterInfo()
      .accounts({
        registrar,
        voter,
      })
      .signers([])
      .simulate()

    // console.log(JSON.stringify(response, undefined, 2))
    const voterInfoData = response.events[0].data;
    assert.isTrue(voterInfoData.depositEntries[0] != null)
    assert.isTrue(voterInfoData.depositEntries[1] != null)
    assert.isTrue(voterInfoData.depositEntries[2] != null)
    assert.isTrue(voterInfoData.depositEntries[3] != null)
    assert.isTrue(voterInfoData.depositEntries[4] != null)
  });
});
