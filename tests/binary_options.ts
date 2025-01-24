import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { BinaryOptions } from "../target/types/binary_options";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("binary_options", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.binaryOptions as Program<BinaryOptions>;
  
  it("Initialize Market", async () => {
    // We can pick some test values
    const strike = new anchor.BN(25_000_000_000); // e.g. $25.0 with some decimal scheme
    const expiry = new anchor.BN(Date.now() / 1000 + 3600); // 1 hour from now
    const oracleFeed = new PublicKey("11111111111111111111111111111111"); // mock

    // Derive the PDA for the Market
    const [marketPda] = await PublicKey.findProgramAddress(
      [
        Buffer.from("market"),
        provider.wallet.publicKey.toBuffer(),
        strike.toArrayLike(Buffer, "le", 8),
        expiry.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    // Call the instruction
    await program.methods
      .initializeMarket(strike, expiry, oracleFeed)
      .accounts({
        market: marketPda,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Fetch the Market account to verify
    const marketAccount = await program.account.market.fetch(marketPda);
    assert.ok(marketAccount.authority.equals(provider.wallet.publicKey));
    assert.equal(marketAccount.strike.toNumber(), strike.toNumber());
    assert.equal(marketAccount.expiry.toNumber(), expiry.toNumber());
    assert.ok(marketAccount.oracleFeed.equals(oracleFeed));
    assert.equal(marketAccount.resolved, false);
    assert.equal(marketAccount.outcome, null);
  });
});
