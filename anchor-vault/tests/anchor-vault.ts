import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Vault } from "../target/types/vault";
import { assert, expect} from "chai";


describe("anchor-vault", async () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const wallet = anchor.Wallet.local();
  const program = anchor.workspace.Vault as Program<Vault>;
  const [vault_state, vault_state_bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), program.provider.publicKey.toBuffer()],
      program.programId
    );
  const [vault, vault_bump] = anchor.web3.PublicKey.findProgramAddressSync(
    [vault_state.toBuffer()],
    program.programId
  );

  it("Initialize Vault", async () => {
    const tx = await program.methods
      .initialize()
      .accounts({
        signer: program.provider.publicKey,
        vaultState: vault_state,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    try {
      assert.ok(await program.account.vault.fetch(vault_state));
    } catch (err) {
      assert.notOk(err);
    }
  });
  it("Fund vault", async () => {
    const amount = 1000000;
    const dx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: program.provider.publicKey,
        toPubkey: vault,
        lamports: amount,
      })
    );
    dx.recentBlockhash = (
      await program.provider.connection.getRecentBlockhash()
    ).blockhash;
    dx.feePayer = program.provider.publicKey;
    const signedTx = await wallet.signTransaction(dx);
    const txid = await program.provider.connection.sendRawTransaction(
      signedTx.serialize()
    );
    await program.provider.connection.confirmTransaction(txid);
    assert.ok((await program.provider.connection.getBalance(vault)) == amount);
  });

  it("Deposit", async () => {
    const initialVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    const depositAmount = new anchor.BN(100);
    const tx = await program.methods
      .deposit(depositAmount)
      .accounts({
        signer: program.provider.publicKey,
        vaultState: vault_state,
        vault: vault,
      })
      .rpc();
    const finalVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    assert.equal(
      finalVaultBalance - initialVaultBalance,
      depositAmount.toNumber()
    );
  });

  it("Withdraw", async () => {
    const initialVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    const withdrawAmount = new anchor.BN(50);
    const tx = await program.methods
      .withdraw(withdrawAmount)
      .accounts({
        signer: program.provider.publicKey,
        vaultState: vault_state,
        vault: vault,
      })
      .rpc();
    const finalVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    assert.equal(
      initialVaultBalance - finalVaultBalance,
      withdrawAmount.toNumber()
    );
  });

  it("Close vault", async () => {
    const initialVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    const tx = await program.methods
      .closeVault()
      .accounts({
        signer: program.provider.publicKey,
        vaultState: vault_state,
        vault: vault,
      })
      .rpc();
    const finalVaultBalance = await program.provider.connection.getBalance(
      vault
    );
    assert.equal(finalVaultBalance, 0);
    try {
      const vaultAccount = await program.account.vault.fetch(vault_state);
    } catch (err) {
      assert.ok(err);
    }
  });
});
