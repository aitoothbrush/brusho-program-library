import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { SendTransactionError, Commitment, Connection, PublicKey, Keypair, TransactionInstruction, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import { VoterStakeRegistry } from "../../target/types/voter_stake_registry";
import { CircuitBreaker } from "../../target/types/circuit_breaker";
import { assert } from "chai";
import { createMint, mintTo, getAccount, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { MintMaxVoteWeightSource, MintMaxVoteWeightSourceType, withCreateRealm, withCreateTokenOwnerRecord } from "@solana/spl-governance";

// Configure the client to use the local cluster.
anchor.setProvider(anchor.AnchorProvider.env());

export const GOV_PROGRAM_ID = new PublicKey("GovernanceProgramTest1111111111111111111111");

export const VSR_PROGRAM = anchor.workspace.VoterStakeRegistry as Program<VoterStakeRegistry>;
export const CIRCUIT_BREAKER_PROGRAM = anchor.workspace.CircuitBreaker as Program<CircuitBreaker>;

export const CONNECTION: Connection = anchor.getProvider().connection;

export const SECS_PER_DAY = new anchor.BN(86_400);
export const SECS_PER_YEAR = SECS_PER_DAY.muln(365);
export const EXP_SCALE = new anchor.BN("1000000000000000000");
export const TOTAL_REWARD_AMOUNT = new anchor.BN("770000000000000"); // 770M
export const FULL_REWARD_PERMANENTLY_LOCKED_FLOOR = new anchor.BN("195000000000000"); // 195M

/// Seconds in one month.
export const SECS_PER_MONTH = SECS_PER_DAY.muln(365).divn(12);

export async function assertThrowsAnchorError(
  codeName: String,
  func: () => Promise<any>,
  callback?: (anchorErr: AnchorError) => undefined,
  logFlag?: boolean) {
  try {
    await func();
  } catch (e) {
    if (logFlag) {
      console.log(e)
    }

    assert.isTrue(e instanceof AnchorError);
    const anchorErr: AnchorError = e;
    assert.strictEqual(anchorErr.error.errorCode.code, codeName);

    if (callback) {
      callback(anchorErr)
    }
    return
  }

  throw "No AnchorError throws"
}

export async function assertThrowsSendTransactionError(
  messagePart: string,
  func: () => Promise<any>,
  callback?: (sendTxErr: SendTransactionError) => undefined,
  logFlag?: boolean) {
  try {
    await func();
  } catch (e) {
    if (logFlag) {
      console.log(e)
    }

    assert.isTrue(e instanceof SendTransactionError);
    const sendTxErr: SendTransactionError = e;
    assert.isTrue(sendTxErr.message.indexOf(messagePart) != 0)

    if (callback) {
      callback(sendTxErr)
    }
    return
  }

  throw "No SendTransactionError throws"
}

export function delay(ms: number) {
    return new Promise( resolve => setTimeout(resolve, ms) );
}


// this airdrops sol to an address
export async function airdropSol(publicKey, amount) {
  let airdropTx = await anchor.getProvider().connection.requestAirdrop(publicKey, amount);
  await confirmTransaction(airdropTx);
}

export async function confirmTransaction(tx) {
  const latestBlockHash = await anchor.getProvider().connection.getLatestBlockhash();
  await anchor.getProvider().connection.confirmTransaction({
    blockhash: latestBlockHash.blockhash,
    lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    signature: tx,
  });
}

export async function newSigner(): Promise<Keypair> {
  const signer = Keypair.generate();
  await airdropSol(signer.publicKey, 10e9);

  return signer;
}

export async function newMint(signer) {
  const mint = await createMint(CONNECTION, signer, signer.publicKey, signer.publicKey, 6, Keypair.generate());
  return mint;
}

export async function newTokenAccount(mint: PublicKey, receiverWallet: Keypair): Promise<PublicKey> {
  return await createAssociatedTokenAccount(CONNECTION, receiverWallet, mint, receiverWallet.publicKey);
}

export async function mintTokenToWallet(mint: PublicKey, mintAuthority: Keypair, receiverWallet: PublicKey, amount: anchor.BN): Promise<PublicKey> {
  const receiverAccount = await getOrCreateAssociatedTokenAccount(CONNECTION, mintAuthority, mint, receiverWallet);
  await mintTo(CONNECTION, mintAuthority, mint, receiverAccount.address, mintAuthority, amount.toNumber())
  return receiverAccount.address;
}

export async function mintTokenToAccount(mint: PublicKey, mintAuthority: Keypair, tokenAccount: PublicKey, amount: anchor.BN) {
  await mintTo(CONNECTION, mintAuthority, mint, tokenAccount, mintAuthority, amount.toNumber(), undefined);
}

export async function getTokenAccount(address: PublicKey) {
  return await getAccount(CONNECTION, address);
}


export async function createRealm(authority: Keypair): Promise<[PublicKey, PublicKey, PublicKey]> {
  const mint = await newMint(authority);
  const councilMint = await newMint(authority);
  // log(`council mint: ${councilMint}`);

  // create realm
  let instructions: TransactionInstruction[] = [];
  let realm = await withCreateRealm(
    instructions,
    GOV_PROGRAM_ID,
    3,
    `BrushO ${new Date().valueOf()}`,
    authority.publicKey,
    mint,
    authority.publicKey,
    councilMint,
    new MintMaxVoteWeightSource({ type: MintMaxVoteWeightSourceType.SupplyFraction, value: new anchor.BN(1e9) }),
    new anchor.BN(1e10),
    undefined,
    undefined
  );
  const transaction = new Transaction().add(...instructions);
  await sendAndConfirmTransaction(
    CONNECTION,
    transaction,
    [authority],
  );

  // log(`realm: ${realm}`)
  return [mint, councilMint, realm];
}

export async function createTokenOwnerRecord(realm: PublicKey, governingTokenMint: PublicKey, voterAuthority: Keypair): Promise<PublicKey> {
  // create realm
  let instructions: TransactionInstruction[] = [];
  let tokenOwnerRecord = await withCreateTokenOwnerRecord(
    instructions,
    GOV_PROGRAM_ID,
    3,
    realm,
    voterAuthority.publicKey,
    governingTokenMint,
    voterAuthority.publicKey,
  );
  const transaction = new Transaction().add(...instructions);
  await sendAndConfirmTransaction(
    CONNECTION,
    transaction,
    [voterAuthority],
  );

  // log(`realm: ${realm}`)
  return tokenOwnerRecord;
}

export async function createRegistrar(
  realm: PublicKey,
  realmAuthority: Keypair,
  governingTokenMint: PublicKey,
  votingConfig: VotingConfig,
  depositConfig: DepositConfig,
  circuit_breaker_threshold: anchor.BN,
  payer: Keypair,
): Promise<[PublicKey, number, PublicKey, PublicKey, PublicKey]> {
  const registrarSeeds = [realm.toBytes(), Buffer.from("registrar"), governingTokenMint.toBytes()];
  const [registrar, registrarBump] = anchor.web3.PublicKey.findProgramAddressSync(registrarSeeds, VSR_PROGRAM.programId);

  const vault = getAssociatedTokenAddressSync(governingTokenMint, registrar, true);
  const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), governingTokenMint.toBytes()];
  const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  await VSR_PROGRAM.methods.createRegistrar(
    registrarBump,
    votingConfig,
    depositConfig,
    circuit_breaker_threshold
  ).accounts({
    registrar,
    realm: realm,
    vault,
    maxVoterWeightRecord,
    circuitBreaker,
    governanceProgramId: GOV_PROGRAM_ID,
    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId,
    realmGoverningTokenMint: governingTokenMint,
    realmAuthority: realmAuthority.publicKey,
    payer: payer.publicKey,
  }).signers([payer, realmAuthority])
    .rpc()

  return [registrar, registrarBump, vault, circuitBreaker, maxVoterWeightRecord];
}

export async function createVoter(
  realm: PublicKey,
  governingTokenMint: PublicKey,
  registrar: PublicKey,
  payer: Keypair
): Promise<[Keypair, PublicKey, PublicKey, PublicKey, PublicKey]> {
  let voterAuthority = await newSigner();

  const voterSeeds = [registrar.toBytes(), Buffer.from("voter"), voterAuthority.publicKey.toBytes()];
  let [voter, voterBump] = anchor.web3.PublicKey.findProgramAddressSync(voterSeeds, VSR_PROGRAM.programId);

  const voterWeightRecordSeeds = [registrar.toBytes(), Buffer.from("voter-weight-record"), voterAuthority.publicKey.toBytes()];
  let [voterWeightRecord, voterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(voterWeightRecordSeeds, VSR_PROGRAM.programId);

  const vaultSeeds = [voter.toBytes(), TOKEN_PROGRAM_ID.toBytes(), governingTokenMint.toBytes()];
  let [vault, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(vaultSeeds, ASSOCIATED_TOKEN_PROGRAM_ID);
  await VSR_PROGRAM.methods.createVoter(
    voterBump,
    voterWeightRecordBump,
  ).accounts({
    registrar,
    governingTokenMint,
    voter,
    voterAuthority: voterAuthority.publicKey,
    vault,
    voterWeightRecord,
    payer: payer.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  }).signers([voterAuthority, payer])
    .rpc()

  const tokenOwnerRecord = await createTokenOwnerRecord(realm, governingTokenMint, voterAuthority);
  return [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord];
}

export async function fastup(registrar: PublicKey, realmAuthority: Keypair, seconds: anchor.BN, commitment: Commitment = "processed") {
  const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
  const currTimeOffset = registrarData.timeOffset;

  await VSR_PROGRAM.methods.setTimeOffset(currTimeOffset.add(seconds))
    .accounts({ registrar, realmAuthority: realmAuthority.publicKey })
    .signers([realmAuthority]).rpc({commitment});
}

export type DepositConfig = {
  ordinaryDepositMinLockupDuration: LockupTimeDuration,
  nodeDepositLockupDuration: LockupTimeDuration,
  nodeSecurityDeposit: anchor.BN,
}

export type VotingConfig = {
  baselineVoteWeightScaledFactor: anchor.BN,
  maxExtraLockupVoteWeightScaledFactor: anchor.BN,
  lockupSaturationSecs: anchor.BN,
}

export function defaultVotingConfig() {
  return {
    baselineVoteWeightScaledFactor: new anchor.BN(1e9),
    maxExtraLockupVoteWeightScaledFactor: new anchor.BN(0),
    lockupSaturationSecs: new anchor.BN(86400),
  };
}

export function defaultDepositConfig() {
  return {
    ordinaryDepositMinLockupDuration: lockupDayily(15),
    nodeDepositLockupDuration: lockupMonthly(6),
    nodeSecurityDeposit: new anchor.BN(10000 * (1e6)),
  };
}

export type LockupTimeDuration = { periods: anchor.BN, unit: { day: {} } | { month: {} }, filler: number[] };
export function newLockupTimeDuration(periods: anchor.BN, unit: 'day' | 'month'): LockupTimeDuration {
  let _unit;
  if (unit === 'day') {
    _unit = { day: {} };
  } else {
    _unit = { month: {} };
  }

  return {
    periods: periods,
    unit: _unit,
    filler: [0, 0, 0, 0, 0, 0, 0]
  };
}

export function lockupDayily(periods: number): LockupTimeDuration {
  return newLockupTimeDuration(new anchor.BN(periods), 'day');
}

export function lockupMonthly(periods: number): LockupTimeDuration {
  return newLockupTimeDuration(new anchor.BN(periods), 'month');
}

export function lockupTimeDurationSeconds(lockupTimeDuration: LockupTimeDuration): anchor.BN {
  if ((lockupTimeDuration.unit as any).day != undefined) {
    return SECS_PER_DAY.mul(lockupTimeDuration.periods);
  } else {
    return SECS_PER_MONTH.mul(lockupTimeDuration.periods);
  }
}