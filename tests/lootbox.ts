// localhost test command, requires docker
//sbv2 anchor test --keypair ~/.config/solana/id.json -s

import * as anchor from "@project-serum/anchor"
import { Program } from "@project-serum/anchor"
import { Lootbox } from "../target/types/lootbox"
import { PublicKey, SYSVAR_RENT_PUBKEY } from "@solana/web3.js"
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  Account,
  TOKEN_PROGRAM_ID,
  mintToChecked,
} from "@solana/spl-token"
import {
  SwitchboardTestContext,
  promiseWithTimeout,
} from "@switchboard-xyz/sbv2-utils"
import * as sbv2 from "@switchboard-xyz/switchboard-v2"
import { assert, expect } from "chai"

const initialAmount = 1000

describe("lootbox", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env())

  const program = anchor.workspace.Lootbox as Program<Lootbox>
  const provider = program.provider as anchor.AnchorProvider
  const wallet = anchor.workspace.Lootbox.provider.wallet
  const connection = anchor.getProvider().connection

  let switchboard: SwitchboardTestContext

  let userState: PublicKey
  let userStateBump: number

  let lootbox: PublicKey
  let mintAuth: PublicKey
  let mintOne: PublicKey
  let mintTwo: PublicKey
  let mintThree: PublicKey
  let stakeMint: PublicKey
  let stakeTokenAccount: Account

  before(async () => {
    // switchboard testing setup
    switchboard = await SwitchboardTestContext.loadDevnetQueue(
      provider,
      "F8ce7MsckeZAbAGmxjJNetxYXQa9mKr9nnrC3qKubyYy",
      100_000_000
    )

    console.log(switchboard.mint.address.toString())
    // switchboard = await SwitchboardTestContext.loadFromEnv(
    //   program.provider as anchor.AnchorProvider,
    //   undefined,
    //   5_000_000 // .005 wSOL
    // )
    await switchboard.oracleHeartbeat()
    const queueData = await switchboard.queue.loadData()
    console.log(`oracleQueue: ${switchboard.queue.publicKey}`)
    console.log(
      `unpermissionedVrfEnabled: ${queueData.unpermissionedVrfEnabled}`
    )
    console.log(`# of oracles heartbeating: ${queueData.queue.length}`)
    console.log(
      "\x1b[32m%s\x1b[0m",
      `\u2714 Switchboard localnet environment loaded successfully\n`
    )
    ;[lootbox] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("LOOTBOX")],
      program.programId
    )
    ;[mintAuth] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("MINT_AUTH")],
      program.programId
    )
    mintOne = await createMint(connection, wallet.payer, mintAuth, null, 0)
    mintTwo = await createMint(connection, wallet.payer, mintAuth, null, 0)
    mintThree = await createMint(connection, wallet.payer, mintAuth, null, 0)

    stakeMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      1
    )

    stakeTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      stakeMint,
      wallet.publicKey
    )

    await mintToChecked(
      connection,
      wallet.payer,
      stakeMint,
      stakeTokenAccount.address,
      wallet.payer,
      initialAmount,
      1
    )
  })

  it("init lootbox", async () => {
    const tx = await program.methods
      .initLootbox()
      .accounts({
        mintOne,
        mintTwo,
        mintThree,
      })
      .rpc()

    const account = await program.account.lootbox.fetch(lootbox)
    assert.isTrue(account.mintOne.equals(mintOne))
    assert.isTrue(account.mintTwo.equals(mintTwo))
    assert.isTrue(account.mintThree.equals(mintThree))

    console.log(`https://explorer.solana.com/tx/${tx}?cluster=devnet`)
  })

  it("init user", async () => {
    const { unpermissionedVrfEnabled, authority, dataBuffer } =
      await switchboard.queue.loadData()

    // keypair for vrf account
    const vrfKeypair = anchor.web3.Keypair.generate()

    // find PDA used for our client state pubkey
    ;[userState, userStateBump] = anchor.utils.publicKey.findProgramAddressSync(
      [wallet.publicKey.toBytes()],
      program.programId
    )

    // create new vrf acount
    const vrfAccount = await sbv2.VrfAccount.create(switchboard.program, {
      keypair: vrfKeypair,
      authority: userState, // set vrfAccount authority as PDA
      queue: switchboard.queue,
      callback: {
        programId: program.programId,
        accounts: [
          { pubkey: userState, isSigner: false, isWritable: true },
          { pubkey: vrfKeypair.publicKey, isSigner: false, isWritable: false },
          { pubkey: lootbox, isSigner: false, isWritable: false },
          { pubkey: wallet.publicKey, isSigner: false, isWritable: false },
        ],
        ixData: new anchor.BorshInstructionCoder(program.idl).encode(
          "consumeRandomness",
          ""
        ),
      },
    })
    console.log(`Created VRF Account: ${vrfAccount.publicKey}`)

    // create permissionAccount
    const permissionAccount = await sbv2.PermissionAccount.create(
      switchboard.program,
      {
        authority,
        granter: switchboard.queue.publicKey,
        grantee: vrfAccount.publicKey,
      }
    )
    console.log(`Created Permission Account: ${permissionAccount.publicKey}`)

    // If queue requires permissions to use VRF, check the correct authority was provided
    if (!unpermissionedVrfEnabled) {
      if (!wallet.publicKey.equals(authority)) {
        throw new Error(
          `queue requires PERMIT_VRF_REQUESTS and wrong queue authority provided`
        )
      }

      await permissionAccount.set({
        authority: wallet.payer,
        permission: sbv2.SwitchboardPermission.PERMIT_VRF_REQUESTS,
        enable: true,
      })
      console.log(`Set VRF Permissions`)
    }

    const vrfState = await vrfAccount.loadData()
    const queueAccount = new sbv2.OracleQueueAccount({
      program: switchboard.program,
      publicKey: vrfState.oracleQueue,
    })

    const queueState = await queueAccount.loadData()

    const [_permissionAccount, permissionBump] =
      sbv2.PermissionAccount.fromSeed(
        switchboard.program,
        queueState.authority,
        queueAccount.publicKey,
        vrfAccount.publicKey
      )

    const [_programStateAccount, switchboardStateBump] =
      sbv2.ProgramStateAccount.fromSeed(switchboard.program)

    const tx = await program.methods
      .initUser({
        switchboardStateBump: switchboardStateBump,
        vrfPermissionBump: permissionBump,
      })
      .accounts({
        state: userState,
        vrf: vrfAccount.publicKey,
        payer: wallet.pubkey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc()

    console.log(`https://explorer.solana.com/tx/${tx}?cluster=devnet`)
  })

  it("request_randomness", async () => {
    const state = await program.account.userState.fetch(userState)

    const vrfAccount = new sbv2.VrfAccount({
      program: switchboard.program,
      publicKey: state.vrf,
    })
    const vrfState = await vrfAccount.loadData()
    const queueAccount = new sbv2.OracleQueueAccount({
      program: switchboard.program,
      publicKey: vrfState.oracleQueue,
    })
    const queueState = await queueAccount.loadData()
    const [permissionAccount, permissionBump] = sbv2.PermissionAccount.fromSeed(
      switchboard.program,
      queueState.authority,
      queueAccount.publicKey,
      vrfAccount.publicKey
    )
    const [programStateAccount, switchboardStateBump] =
      sbv2.ProgramStateAccount.fromSeed(switchboard.program)

    const tx = await program.methods
      .requestRandomness()
      .accounts({
        state: userState,
        vrf: vrfAccount.publicKey,
        oracleQueue: queueAccount.publicKey,
        queueAuthority: queueState.authority,
        dataBuffer: queueState.dataBuffer,
        permission: permissionAccount.publicKey,
        escrow: vrfState.escrow,
        programState: programStateAccount.publicKey,
        switchboardProgram: switchboard.program.programId,
        payerWallet: switchboard.payerTokenWallet,
        payer: wallet.publicKey,
        recentBlockhashes: anchor.web3.SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
        stakeMint: stakeMint,
        stakeTokenAccount: stakeTokenAccount.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc()

    const result = await awaitCallback(program, userState, 20_000)

    console.log(`VrfClient Result: ${result}`)

    const updated_state = await program.account.userState.fetch(userState)

    console.log(updated_state.mint.toString())
    console.log(updated_state.tokenAccount.toString())

    const account = await getAccount(connection, stakeTokenAccount.address)
    console.log(account.amount.toString())

    console.log(`https://explorer.solana.com/tx/${tx}?cluster=devnet`)
  })

  it("mint_reward", async () => {
    const state = await program.account.userState.fetch(userState)

    const tx = await program.methods
      .mintReward()
      .accounts({
        state: userState,
        mint: state.mint,
        tokenAccount: state.tokenAccount,
        mintAuthority: mintAuth,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: wallet.publicKey,
      })
      .rpc()

    const account = await getAccount(connection, state.tokenAccount)
    console.log(account.amount.toString())

    console.log(`https://explorer.solana.com/tx/${tx}?cluster=devnet`)
  })

  it("request again", async () => {
    const state = await program.account.userState.fetch(userState)

    const vrfAccount = new sbv2.VrfAccount({
      program: switchboard.program,
      publicKey: state.vrf,
    })
    const vrfState = await vrfAccount.loadData()
    const queueAccount = new sbv2.OracleQueueAccount({
      program: switchboard.program,
      publicKey: vrfState.oracleQueue,
    })
    const queueState = await queueAccount.loadData()
    const [permissionAccount, permissionBump] = sbv2.PermissionAccount.fromSeed(
      switchboard.program,
      queueState.authority,
      queueAccount.publicKey,
      vrfAccount.publicKey
    )
    const [programStateAccount, switchboardStateBump] =
      sbv2.ProgramStateAccount.fromSeed(switchboard.program)

    const tx1 = await program.methods
      .requestRandomness()
      .accounts({
        state: userState,
        vrf: vrfAccount.publicKey,
        oracleQueue: queueAccount.publicKey,
        queueAuthority: queueState.authority,
        dataBuffer: queueState.dataBuffer,
        permission: permissionAccount.publicKey,
        escrow: vrfState.escrow,
        programState: programStateAccount.publicKey,
        switchboardProgram: switchboard.program.programId,
        payerWallet: switchboard.payerTokenWallet,
        payer: wallet.publicKey,
        recentBlockhashes: anchor.web3.SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
        stakeMint: stakeMint,
        stakeTokenAccount: stakeTokenAccount.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc()

    const result = await awaitCallback(program, userState, 20_000)

    console.log(`VrfClient Result: ${result}`)

    const tx2 = await program.methods
      .mintReward()
      .accounts({
        state: userState,
        mint: state.mint,
        tokenAccount: state.tokenAccount,
        mintAuthority: mintAuth,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: wallet.publicKey,
      })
      .rpc()
  })
})

async function awaitCallback(
  program: Program<Lootbox>,
  vrfClientKey: anchor.web3.PublicKey,
  timeoutInterval: number,
  errorMsg = "Timed out waiting for VRF Client callback"
) {
  let ws: number | undefined = undefined
  const result: anchor.BN = await promiseWithTimeout(
    timeoutInterval,
    new Promise((resolve: (result: anchor.BN) => void) => {
      ws = program.provider.connection.onAccountChange(
        vrfClientKey,
        async (
          accountInfo: anchor.web3.AccountInfo<Buffer>,
          context: anchor.web3.Context
        ) => {
          const clientState = program.account.userState.coder.accounts.decode(
            "UserState",
            accountInfo.data
          )
          if (clientState.result.gt(new anchor.BN(0))) {
            resolve(clientState.result)
          }
        }
      )
    }).finally(async () => {
      if (ws) {
        await program.provider.connection.removeAccountChangeListener(ws)
      }
      ws = undefined
    }),
    new Error(errorMsg)
  ).finally(async () => {
    if (ws) {
      await program.provider.connection.removeAccountChangeListener(ws)
    }
    ws = undefined
  })

  return result
}
