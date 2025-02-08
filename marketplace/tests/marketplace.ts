import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Marketplace } from "../target/types/marketplace";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import {
  createNft,
  findMasterEditionPda,
  findMetadataPda,
  mplTokenMetadata,
  verifySizedCollectionItem,
} from "@metaplex-foundation/mpl-token-metadata";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  KeypairSigner,
  PublicKey,
  createSignerFromKeypair,
  generateSigner,
  keypairIdentity,
  percentAmount,
} from "@metaplex-foundation/umi";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

describe("marketplace", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Marketplace as Program<Marketplace>;

  const umi = createUmi(provider.connection);

  const payer = provider.wallet as NodeWallet;

  let nftMint: KeypairSigner = generateSigner(umi);
  let collectionMint: KeypairSigner = generateSigner(umi);

  const creatorWallet = umi.eddsa.createKeypairFromSecretKey(
    new Uint8Array(payer.payer.secretKey)
  );
  const creator = createSignerFromKeypair(umi, creatorWallet);
  umi.use(keypairIdentity(creator));
  umi.use(mplTokenMetadata());

  const collection: anchor.web3.PublicKey = new anchor.web3.PublicKey(
    collectionMint.publicKey.toString()
  );

  it("Mint Collection NFT", async () => {
    await createNft(umi, {
      mint: collectionMint,
      name: "GM",
      symbol: "GM",
      uri: "https://arweave.net/123",
      sellerFeeBasisPoints: percentAmount(5.5),
      creators: null,
      collectionDetails: {
        __kind: "V1",
        size: 10,
      },
    }).sendAndConfirm(umi);
    console.log(
      `Created Collection NFT: ${collectionMint.publicKey.toString()}`
    );
  });

  it("Mint NFT", async () => {
    await createNft(umi, {
      mint: nftMint,
      name: "GM",
      symbol: "GM",
      uri: "https://arweave.net/123",
      sellerFeeBasisPoints: percentAmount(5.5),
      creators: null,
      collection: {
        verified: false,
        key: collectionMint.publicKey,
      },
    }).sendAndConfirm(umi);
    console.log(`Created NFT: ${nftMint.publicKey.toString()}`);
  });

  it("verify collection", async () => {
    const collectionMetadata = findMetadataPda(umi, {
      mint: collectionMint.publicKey,
    });
    const collectionMasterEdition = findMasterEditionPda(umi, {
      mint: collectionMint.publicKey,
    });

    const nftMetadata = findMetadataPda(umi, { mint: nftMint.publicKey });
    await verifySizedCollectionItem(umi, {
      metadata: nftMetadata,
      collectionAuthority: creator,
      collectionMint: collectionMint.publicKey,
      collection: collectionMetadata,
      collectionMasterEditionAccount: collectionMasterEdition,
    }).sendAndConfirm(umi);
    console.log("Collection NFT Verified!");
  });
  const marketPlaceName = "Sataa Bajaar";
  const marketFee = 16;
  const [marketPdaAccount, marketPdaAccountBump] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("marketplace"), Buffer.from(marketPlaceName)],
      program.programId
    );
  const [treasuryPda, treasuryBump] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("treasury"), marketPdaAccount.toBuffer()],
      program.programId
    );

  it("Initialize market", async () => {
    try {
      // Pre-fund treasury with some SOL
      await provider.sendAndConfirm(
        new anchor.web3.Transaction().add(
          anchor.web3.SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: treasuryPda,
            lamports: anchor.web3.LAMPORTS_PER_SOL / 10, // 0.1 SOL
          })
        )
      );
      console.log("Funded treasury with 0.1 SOL");

      const tx = await program.methods
        .initializeMarket(marketFee, marketPlaceName)
        .accounts({
          admin: provider.wallet.publicKey,
          marketplace: marketPdaAccount,
          treasury: treasuryPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      console.log("Market initialized with tx:", tx);
      const marketAccount = await program.account.marketPlace.fetch(
        marketPdaAccount
      );
      console.log("Market account:", marketAccount);
    } catch (error) {
      console.error("Error:", error);
      throw error;
    }
  });

  it("List NFT", async () => {
    // Compute listing PDA and vault ATA
    const [listingPda, listingBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [marketPdaAccount.toBuffer(), Buffer.from(nftMint.publicKey)],
        program.programId
      );
    const vault = getAssociatedTokenAddressSync(
      new anchor.web3.PublicKey(nftMint.publicKey),
      listingPda,
      true // allowOwnerOffCurve
    );

    const price = new anchor.BN(1000000);
    const [metadataPda] = findMetadataPda(umi, { mint: nftMint.publicKey });
    const [masterEditionPda] = findMasterEditionPda(umi, {
      mint: nftMint.publicKey,
    });

    const tx = await program.methods
      .list(price)
      .accounts({
        maker: provider.wallet.publicKey,
        marketplace: marketPdaAccount,
        makerMint: nftMint.publicKey,
        makerAta: getAssociatedTokenAddressSync(
          new anchor.web3.PublicKey(nftMint.publicKey),
          provider.wallet.publicKey
        ),
        vault: vault,
        listing: listingPda,
        collectionMint: collectionMint.publicKey,
        metadata: metadataPda,
        masterEditionAccount: masterEditionPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        metadataProgram: TOKEN_2022_PROGRAM_ID,
      })
      .rpc();
    console.log("NFT listed", tx);
  });

  it("Purchase NFT", async () => {
    // For simplicity use the same PDA addresses from listing test.
    const [listingPda, _] = anchor.web3.PublicKey.findProgramAddressSync(
      [marketPdaAccount.toBuffer(), Buffer.from(nftMint.publicKey)],
      program.programId
    );
    const vault = getAssociatedTokenAddressSync(
      new anchor.web3.PublicKey(nftMint.publicKey),
      listingPda,
      true
    );
    const tx = await program.methods
      .purchase()
      .accounts({
        taker: provider.wallet.publicKey,
        maker: provider.wallet.publicKey,
        marketplace: marketPdaAccount,
        makerMint: new anchor.web3.PublicKey(nftMint.publicKey),
        takerAta: getAssociatedTokenAddressSync(
          new anchor.web3.PublicKey(nftMint.publicKey),
          provider.wallet.publicKey
        ),
        vault: vault,
        listing: listingPda,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("NFT purchased", tx);
  });

  it("Delist NFT", async () => {
    // Use the same PDA addresses computed earlier.
    const [listingPda, _] = anchor.web3.PublicKey.findProgramAddressSync(
      [marketPdaAccount.toBuffer(), Buffer.from(nftMint.publicKey)],
      program.programId
    );
    const vault = getAssociatedTokenAddressSync(
      new anchor.web3.PublicKey(nftMint.publicKey),
      listingPda,
      true
    );
    const tx = await program.methods
      .delist()
      .accounts({
        maker: provider.wallet.publicKey,
        marketplace: marketPdaAccount,
        makerMint: new anchor.web3.PublicKey(nftMint.publicKey),
        makerAta: getAssociatedTokenAddressSync(
          new anchor.web3.PublicKey(nftMint.publicKey),
          provider.wallet.publicKey
        ),
        vault: vault,
        listing: listingPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("NFT delisted", tx);
  });
});
