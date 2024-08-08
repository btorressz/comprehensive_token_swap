u  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.ComprehensiveTokenSwap as anchor.Program<ComprehensiveTokenSwap>;
  
import type { ComprehensiveTokenSwap } from "../target/types/comprehensive_token_swap";
se anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::sysvar::{self, Sysvar};
use anchor_lang::AccountsClose;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer};
use anchor_spl::token::{MintTo, Token};
use solana_program::program_pack::Pack;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

#[tokio::test]
async fn test_add_liquidity() {
    let program_id = Pubkey::new_unique();
    let swap = Keypair::new();
    let user = Keypair::new();

    let mut pc = ProgramTest::new(
        "comprehensive_token_swap",
        program_id,
        processor!(comprehensive_token_swap::comprehensive_token_swap::process),
    );

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;
    
    // Create token mints and token accounts
    let token_a_mint = Keypair::new();
    let token_b_mint = Keypair::new();
    let user_token_a_account = Keypair::new();
    let user_token_b_account = Keypair::new();
    let pool_token_a_account = Keypair::new();
    let pool_token_b_account = Keypair::new();
    
    // Create and mint tokens
    create_mint(&mut banks_client, &payer, &token_a_mint, &payer.pubkey()).await.unwrap();
    create_mint(&mut banks_client, &payer, &token_b_mint, &payer.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &user_token_a_account, &token_a_mint.pubkey(), &user.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &user_token_b_account, &token_b_mint.pubkey(), &user.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &pool_token_a_account, &token_a_mint.pubkey(), &swap.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &pool_token_b_account, &token_b_mint.pubkey(), &swap.pubkey()).await.unwrap();
    
    mint_tokens(&mut banks_client, &payer, &token_a_mint.pubkey(), &user_token_a_account.pubkey(), 10000).await.unwrap();
    mint_tokens(&mut banks_client, &payer, &token_b_mint.pubkey(), &user_token_b_account.pubkey(), 10000).await.unwrap();
    
    // Initialize the swap state
    let initialize_ix = comprehensive_token_swap::instruction::initialize(
        program_id,
        swap.pubkey(),
        user.pubkey(),
        3,
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &swap], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Add liquidity
    let amount_a = 1000;
    let amount_b = 1000;

    let add_liquidity_ix = comprehensive_token_swap::instruction::add_liquidity(
        program_id,
        swap.pubkey(),
        user.pubkey(),
        user_token_a_account.pubkey(),
        user_token_b_account.pubkey(),
        pool_token_a_account.pubkey(),
        pool_token_b_account.pubkey(),
        amount_a,
        amount_b,
    );

    transaction = Transaction::new_with_payer(&[add_liquidity_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Validate the results (e.g., pool reserves updated, events emitted)
    let pool_account = banks_client.get_account(pool_token_a_account.pubkey()).await.unwrap().unwrap();
    let pool = TokenAccount::unpack(&pool_account.data).unwrap();
    assert_eq!(pool.amount, amount_a);

    let pool_account = banks_client.get_account(pool_token_b_account.pubkey()).await.unwrap().unwrap();
    let pool = TokenAccount::unpack(&pool_account.data).unwrap();
    assert_eq!(pool.amount, amount_b);
}

#[tokio::test]
async fn test_simple_swap() {
    let program_id = Pubkey::new_unique();
    let swap = Keypair::new();
    let user = Keypair::new();

    let mut pc = ProgramTest::new(
        "comprehensive_token_swap",
        program_id,
        processor!(comprehensive_token_swap::comprehensive_token_swap::process),
    );

    let (mut banks_client, payer, recent_blockhash) = pc.start().await;
    
    // Create token mints and token accounts
    let token_a_mint = Keypair::new();
    let token_b_mint = Keypair::new();
    let user_token_a_account = Keypair::new();
    let user_token_b_account = Keypair::new();
    let pool_token_a_account = Keypair::new();
    let pool_token_b_account = Keypair::new();
    
    // Create and mint tokens
    create_mint(&mut banks_client, &payer, &token_a_mint, &payer.pubkey()).await.unwrap();
    create_mint(&mut banks_client, &payer, &token_b_mint, &payer.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &user_token_a_account, &token_a_mint.pubkey(), &user.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &user_token_b_account, &token_b_mint.pubkey(), &user.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &pool_token_a_account, &token_a_mint.pubkey(), &swap.pubkey()).await.unwrap();
    create_token_account(&mut banks_client, &payer, &pool_token_b_account, &token_b_mint.pubkey(), &swap.pubkey()).await.unwrap();
    
    mint_tokens(&mut banks_client, &payer, &token_a_mint.pubkey(), &user_token_a_account.pubkey(), 10000).await.unwrap();
    mint_tokens(&mut banks_client, &payer, &token_b_mint.pubkey(), &user_token_b_account.pubkey(), 10000).await.unwrap();
    
    // Initialize the swap state
    let initialize_ix = comprehensive_token_swap::instruction::initialize(
        program_id,
        swap.pubkey(),
        user.pubkey(),
        3,
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &swap], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Add liquidity
    let amount_a = 1000;
    let amount_b = 1000;

    let add_liquidity_ix = comprehensive_token_swap::instruction::add_liquidity(
        program_id,
        swap.pubkey(),
        user.pubkey(),
        user_token_a_account.pubkey(),
        user_token_b_account.pubkey(),
        pool_token_a_account.pubkey(),
        pool_token_b_account.pubkey(),
        amount_a,
        amount_b,
    );

    transaction = Transaction::new_with_payer(&[add_liquidity_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Perform a simple swap
    let swap_amount = 500;
    let min_amount_b = 400;  // Adjust based on your calculation logic

    let simple_swap_ix = comprehensive_token_swap::instruction::simple_swap(
        program_id,
        swap.pubkey(),
        user.pubkey(),
        user_token_a_account.pubkey(),
        user_token_b_account.pubkey(),
        pool_token_a_account.pubkey(),
        pool_token_b_account.pubkey(),
        swap_amount,
        min_amount_b,
    );

    transaction = Transaction::new_with_payer(&[simple_swap_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &user], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Validate the results (e.g., pool reserves updated, events emitted)
    let pool_account = banks_client.get_account(pool_token_a_account.pubkey()).await.unwrap().unwrap();
    let pool = TokenAccount::unpack(&pool_account.data).unwrap();
    assert_eq!(pool.amount, 1000 + swap_amount); // initial amount + swapped amount

    let pool_account = banks_client.get_account(pool_token_b_account.pubkey()).await.unwrap().unwrap();
    let pool = TokenAccount::unpack(&pool_account.data).unwrap();
    assert!(pool.amount < 1000); // initial amount - swapped amount (considering fee)
}

// Helper functions for test setup
async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: &Keypair,
    authority: &Pubkey,
) -> Result<(), BanksClientError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_space = Mint::LEN;
    let mint_rent = rent.minimum_balance(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        mint_space as u64,
        &spl_token::id(),
    );

    let initialize_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        authority,
        None,
        0,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[create_mint_ix, initialize_mint_ix],
        Some(&payer.pubkey()),
        &[payer, mint],
        banks_client.get_recent_blockhash().await.unwrap(),
    );

    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    token_account: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), BanksClientError> {
    let rent = banks_client.get_rent().await.unwrap();
    let token_account_space = TokenAccount::LEN;
    let token_account_rent = rent.minimum_balance(token_account_space);

    let create_token_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        token_account_rent,
        token_account_space as u64,
        &spl_token::id(),
    );

    let initialize_token_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[create_token_account_ix, initialize_token_account_ix],
        Some(&payer.pubkey()),
        &[payer, token_account],
        banks_client.get_recent_blockhash().await.unwrap(),
    );

    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        token_account,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &[payer],
        banks_client.get_recent_blockhash().await.unwrap(),
    );

    banks_client.process_transaction(transaction).await?;
    Ok(())
}
