import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, newSigner, VSR_PROGRAM, fastup, SECS_PER_MONTH, lockupDayily, createRealm, createVoter, newTokenAccount, mintTokenToAccount, createRegistrar, defaultDepositConfig, defaultVotingConfig } from "./helper";
import { Account, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


// describe("node_release_deposit!", () => {
//   let authority: web3.Keypair;
//   let mint: web3.PublicKey;
//   let councilMint: web3.PublicKey;
//   let realm: web3.PublicKey;
//   let registrar: web3.PublicKey;
//   let voterAuthority: web3.Keypair;
//   let voter: web3.PublicKey;
//   let voterWeightRecord: web3.PublicKey;
//   let vault: web3.PublicKey;
//   let tokenOwnerRecord: web3.PublicKey;
//   let depositToken: web3.PublicKey;

//   let nodeSecurityDeposit: anchor.BN;

//   before(async () => {
//     authority = await newSigner();
//     [mint, councilMint, realm] = await createRealm(authority);
//     // create registrar
//     [registrar] = await createRegistrar(realm, authority, mint, defaultVotingConfig(), defaultDepositConfig(), authority);
//     [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
//     depositToken = await newTokenAccount(mint, voterAuthority);
//     nodeSecurityDeposit = defaultDepositConfig().nodeSecurityDeposit;

//     // console.log(`authority: ${authority.publicKey.toBase58()}`)
//     // console.log(`mint: ${mint.toBase58()}`)
//     // console.log(`councilMint: ${councilMint.toBase58()}`)
//     // console.log(`realm: ${realm.toBase58()}`)
//     // console.log(`registrar: ${registrar.toBase58()}`)
//     // console.log(`voterAuthority: ${voterAuthority.publicKey.toBase58()}`)
//     // console.log(`voter: ${voter.toBase58()}`)
//     // console.log(`voterWeightRecord: ${voterWeightRecord.toBase58()}`)
//     // console.log(`vault: ${vault.toBase58()}`)
//     // console.log(`tokenOwnerRecord: ${tokenOwnerRecord.toBase58()}`)
//     // console.log(`depositToken: ${depositToken.toBase58()}`)
//   })


//   it("with_incorrect_registrar_should_fail", async () => {
//     let [invalidRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), authority);

//     await assertThrowsAnchorError('ConstraintSeeds', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(1)
//         .accounts({
//           registrar: invalidRegistrar,
//           voter,
//           voterAuthority: voterAuthority.publicKey,
//         }).signers([voterAuthority])
//         .rpc();
//     },
//       (anchorErr) => {
//         if (anchorErr.error.comparedValues) {
//           assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
//         }
//       },
//     );
//   });

//   it("with_incorrect_voter_authority_should_fail", async () => {
//     const invalidVoterAuthority = await newSigner();

//     await assertThrowsAnchorError('ConstraintSeeds', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(1)
//         .accounts({
//           registrar,
//           voter,
//           voterAuthority: invalidVoterAuthority.publicKey,
//         }).signers([invalidVoterAuthority])
//         .rpc();
//     },
//       (anchorErr) => {
//         if (anchorErr.error.comparedValues) {
//           assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
//         }
//       },
//     );
//   });

//   it("release_before deposit_should_fail", async () => {
//     await assertThrowsAnchorError('InactiveDepositEntry', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(1)
//         .accounts({
//           registrar,
//           voter,
//           voterAuthority: voterAuthority.publicKey,
//         }).signers([voterAuthority])
//         .rpc();
//     },
//       undefined,
//       false
//     );
//   });

//   it("with_active_target_entry_index_should_fail", async () => {
//     // become node
//     await mintTokenToAccount(mint, authority, depositToken, nodeSecurityDeposit);
//     await VSR_PROGRAM.methods
//       .nodeDeposit()
//       .accounts({
//         registrar,
//         voter,
//         vault,
//         depositToken,
//         depositAuthority: voterAuthority.publicKey,
//         tokenProgram: TOKEN_PROGRAM_ID
//       }).signers([voterAuthority])
//       .rpc();

//     // ordinary deposit at index 1
//     const depositAmount = new anchor.BN(1e9);
//     const targetDepositEntryIndex = 1;
//     await mintTokenToAccount(mint, authority, depositToken, depositAmount);
//     await VSR_PROGRAM.methods
//       .ordinaryDeposit(targetDepositEntryIndex, depositAmount, lockupDayily(15))
//       .accounts({
//         registrar,
//         voter,
//         vault,
//         depositToken,
//         depositAuthority: voterAuthority.publicKey,
//         tokenProgram: TOKEN_PROGRAM_ID
//       }).signers([voterAuthority])
//       .rpc();

//     await assertThrowsAnchorError('ActiveDepositEntryIndex', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(targetDepositEntryIndex)
//         .accounts({
//           registrar,
//           voter,
//           voterAuthority: voterAuthority.publicKey,
//         }).signers([voterAuthority])
//         .rpc();
//     },
//       undefined,
//       false
//     );
//   });

//   it("release_before_lockup_expired_should_fail", async () => {
//     const targetDepositEntryIndex = 2;
//     await assertThrowsAnchorError('NodeDepositUnreleasableAtPresent', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(targetDepositEntryIndex)
//         .accounts({
//           registrar,
//           voter,
//           voterAuthority: voterAuthority.publicKey,
//         }).signers([voterAuthority])
//         .rpc();
//     },
//       undefined,
//       false
//     );

//     // fastup 5 month
//     await fastup(registrar, authority, SECS_PER_MONTH.muln(5));
//     await assertThrowsAnchorError('NodeDepositUnreleasableAtPresent', async () => {
//       await VSR_PROGRAM.methods
//         .nodeReleaseDeposit(targetDepositEntryIndex)
//         .accounts({
//           registrar,
//           voter,
//           voterAuthority: voterAuthority.publicKey,
//         }).signers([voterAuthority])
//         .rpc();
//     },
//       undefined,
//       false
//     );

//   });

//   it("verify_node_release_deposit", async () => {
//     const targetDepositEntryIndex = 2;
//     // fastup 1 month more
//     await fastup(registrar, authority, SECS_PER_MONTH)
//     await VSR_PROGRAM.methods
//       .nodeReleaseDeposit(targetDepositEntryIndex)
//       .accounts({
//         registrar,
//         voter,
//         voterAuthority: voterAuthority.publicKey,
//       }).signers([voterAuthority])
//       .rpc();

//     const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
//     const nodeDepositEntry = voterData.deposits.at(0);
//     // console.log(JSON.stringify(originDepositEntry, undefined, 2))
//     assert.isFalse(nodeDepositEntry.isActive)

//     const targetDepositEntry = voterData.deposits.at(targetDepositEntryIndex);
//     // console.log(JSON.stringify(targetDepositEntry, undefined, 2))
//     assert.isTrue(targetDepositEntry.isActive)
//     assert.isTrue(targetDepositEntry.amountDepositedNative.eq(nodeSecurityDeposit))
//     assert.isTrue(targetDepositEntry.amountInitiallyLockedNative.eq(nodeSecurityDeposit))
//     assert.isTrue(targetDepositEntry.lockup.kind.monthly != undefined) // assert lockup kind is Monthly
//     assert.equal(targetDepositEntry.lockup.kind.monthly![0], 6) // assert periods of lockup time duration is 6
//   });
// });
