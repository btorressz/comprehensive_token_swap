//Local Version
import * as anchor from "@project-serum/anchor";
import * as web3 from "@solana/web3.js";
import BN from "bn.js"; // For handling large numbers

// Configure the client to use the local cluster (local validator or network)
anchor.setProvider(anchor.AnchorProvider.env());

// Define the program interface based on your deployed program ID and IDL
const program = anchor.workspace.ComprehensiveTokenSwap as anchor.Program<any>; // Replace 'any' with  program's IDL type

describe("Comprehensive Token Swap Tests", () => {
  
  // Test for initializing the token swap pool
  it("Initialize the Token Swap Pool", async () => {
    const poolAccountKp = new web3.Keypair();
    const [poolAuthority, _] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("token_swap_pool")],
      program.programId
    );
    
    const txHash = await program.methods
      .initializePool(0) // bump seed
      .accounts({
        pool: poolAccountKp.publicKey,
        poolAuthority: poolAuthority,
        admin: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([poolAccountKp])
      .rpc();

    console.log(`Token Swap Pool initialized. Tx: ${txHash}`);
    await program.provider.connection.confirmTransaction(txHash);
  });

  // Test for adding liquidity to the pool
  it("Add Liquidity", async () => {
    const poolAccountKey = new web3.PublicKey("<your-pool-account>"); // Replace with the actual pool account

    const txHash = await program.methods
      .addLiquidity(new BN(1000), new BN(2000)) // Amounts for token A and token B
      .accounts({
        pool: poolAccountKey,
        user: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log(`Liquidity added. Tx: ${txHash}`);
    await program.provider.connection.confirmTransaction(txHash);
  });

  // Test for swapping tokens
  it("Swap Tokens", async () => {
    const poolAccountKey = new web3.PublicKey("<your-pool-account>"); // Replace with the actual pool account

    const txHash = await program.methods
      .swapTokens(new BN(500), new BN(400)) // Swap token A for token B
      .accounts({
        pool: poolAccountKey,
        user: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log(`Tokens swapped. Tx: ${txHash}`);
    await program.provider.connection.confirmTransaction(txHash);
  });

  // Test for performing a flash swap
  it("Flash Swap", async () => {
    const poolAccountKey = new web3.PublicKey("<your-pool-account>"); // Replace with the actual pool account
    const targetContract = new web3.Keypair(); // Contract to interact with during flash swap

    const txHash = await program.methods
      .flashSwap(new BN(1000), targetContract.publicKey)
      .accounts({
        pool: poolAccountKey,
        user: program.provider.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([targetContract])
      .rpc();

    console.log(`Flash swap executed. Tx: ${txHash}`);
    await program.provider.connection.confirmTransaction(txHash);
  });
});
