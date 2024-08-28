use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("Hng6hDtW2VtYjJwx5RUH7zyuKpQFZMBhmkj17bNTVT18");

#[program]
mod comprehensive_token_swap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_rate: u64) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        swap.fee_rate = fee_rate;
        swap.paused = false;
        Ok(())
    }

    pub fn add_liquidity(ctx: Context<AddLiquidityCtx>, amount_a: u64, amount_b: u64) -> Result<()> {
        let transfer_ctx_a = ctx.accounts.into_transfer_to_pool_context_a();
        let transfer_ctx_b = ctx.accounts.into_transfer_to_pool_context_b();

        token::transfer(transfer_ctx_a, amount_a)?;
        token::transfer(transfer_ctx_b, amount_b)?;

        let pool = &mut ctx.accounts.pool;
        pool.token_a_reserve += amount_a;
        pool.token_b_reserve += amount_b;

        emit!(AddLiquidityEvent {
            user: *ctx.accounts.user.to_account_info().key,
            amount_a,
            amount_b,
        });

        Ok(())
    }

   pub fn simple_swap(ctx: Context<SimpleSwapCtx>, amount_a: u64, min_amount_b: u64) -> Result<()> {
             // Extract the necessary values from the context
              let transfer_ctx_to_pool = ctx.accounts.into_transfer_to_pool_context();
              let transfer_ctx_to_user = ctx.accounts.into_transfer_to_user_context();
              let transfer_ctx_fee = ctx.accounts.into_transfer_fee_context();

              // Mutate before transfer to calculate the amount and fee
              let pool = &ctx.accounts.pool; // Immutable borrow to calculate
              let swap = &ctx.accounts.swap; // Immutable borrow to calculate
    
             let amount_b = get_swap_amount(amount_a, pool.token_a_reserve, pool.token_b_reserve)?;
             let fee = calculate_fee(amount_b, swap.fee_rate);

            require!(amount_b >= min_amount_b, ErrorCode::SlippageExceeded);

              // Perform all transfers
             token::transfer(transfer_ctx_to_pool, amount_a)?;
             token::transfer(transfer_ctx_to_user, amount_b - fee)?;
             token::transfer(transfer_ctx_fee, fee)?;

    // Now perform mutable borrow after transfers
    let pool = &mut ctx.accounts.pool;
    pool.token_a_reserve += amount_a;
    pool.token_b_reserve -= amount_b;

    emit!(SimpleSwapEvent {
        user: *ctx.accounts.user.to_account_info().key,
        amount_a,
        amount_b,
        fee,
    });

    Ok(())
}


    // Limit Order Functions
    pub fn place_limit_order(
        ctx: Context<PlaceLimitOrderCtx>,
        amount_a: u64,
        target_price: u64,
        expiration: i64,
        partial_fill: bool,
    ) -> Result<()> {
        let order = &mut ctx.accounts.order;
        order.user = *ctx.accounts.user.key;
        order.token_a_reserve = amount_a;
        order.target_price = target_price;
        order.expiration = expiration;
        order.partial_fill = partial_fill;

        emit!(LimitOrderPlaced {
            user: *ctx.accounts.user.key,
            amount_a,
            target_price,
            expiration,
            partial_fill,
        });

        Ok(())
    }

    pub fn execute_limit_order(ctx: Context<ExecuteLimitOrderCtx>, current_price: u64) -> Result<()> {
        let order = &mut ctx.accounts.order;

        let current_time = Clock::get()?.unix_timestamp;
        require!(order.expiration >= current_time, ErrorCode::OrderExpired);
        require!(current_price >= order.target_price, ErrorCode::PriceNotMet);

        let amount_a = if order.partial_fill {
            let available_amount = ctx.accounts.pool.token_a_reserve;
            std::cmp::min(available_amount, order.token_a_reserve)
        } else {
            order.token_a_reserve
        };

        let pool = &mut ctx.accounts.pool;
        pool.token_a_reserve -= amount_a;
        pool.token_b_reserve += current_price * amount_a;

        emit!(LimitOrderExecuted {
            user: order.user,
            amount_a,
            target_price: order.target_price,
        });

        Ok(())
    }

    // Multi-Token Swap Function
    pub fn multi_token_swap(
        ctx: Context<MultiTokenSwapCtx>,
        src_token: Pubkey,
        dst_token: Pubkey,
        amount: u64,
        min_dst_amount: u64,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;

        let intermediate_amount = route_swap(amount, pool.token_a_reserve, pool.token_b_reserve)?;
        require!(intermediate_amount >= min_dst_amount, ErrorCode::SlippageExceeded);

        let transfer_ctx = ctx.accounts.into_transfer_context();
        token::transfer(transfer_ctx, intermediate_amount)?;

        emit!(MultiTokenSwapEvent {
            user: *ctx.accounts.user.to_account_info().key,
            src_token,
            dst_token,
            amount,
            received: intermediate_amount,
        });

        Ok(())
    }

    // Flash Swap Function
    pub fn flash_swap(ctx: Context<FlashSwapCtx>, amount_a: u64, target_contract: Pubkey) -> Result<()> {
        let initial_balance = ctx.accounts.pool.token_a_reserve;

        let transfer_ctx = ctx.accounts.into_transfer_context();
        token::transfer(transfer_ctx, amount_a)?;

        let ix = solana_program::instruction::Instruction {
            program_id: target_contract,
            accounts: vec![],
            data: vec![],
        };
        solana_program::program::invoke_signed(&ix, &[], &[])?;

        let pool = &mut ctx.accounts.pool;
        let current_balance = pool.token_a_reserve;
        require!(current_balance >= initial_balance, ErrorCode::FlashSwapFailed);

        emit!(FlashSwapEvent {
            user: *ctx.accounts.user.to_account_info().key,
            amount_a,
            target_contract,
        });

        Ok(())
    }
}

// Context Structs
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8 + 1)]
    pub swap: Account<'info, SwapState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddLiquidityCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user_token_a_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_b_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_a_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_b_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

// Implement the helper functions for AddLiquidityCtx
impl<'info> AddLiquidityCtx<'info> {
    fn into_transfer_to_pool_context_a(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_a_account.to_account_info(),
                to: self.pool_token_a_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }

    fn into_transfer_to_pool_context_b(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_b_account.to_account_info(),
                to: self.pool_token_b_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}

#[derive(Accounts)]
pub struct SimpleSwapCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub swap: Account<'info, SwapState>,
    #[account(mut)]
    pub user_token_a_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_b_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_a_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_b_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

// Implement the helper functions for SimpleSwapCtx
impl<'info> SimpleSwapCtx<'info> {
    fn into_transfer_to_pool_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_a_account.to_account_info(),
                to: self.pool_token_a_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }

    fn into_transfer_to_user_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.pool_token_b_account.to_account_info(),
                to: self.user_token_b_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }

    fn into_transfer_fee_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.pool_token_b_account.to_account_info(),
                to: self.user_token_b_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}

#[derive(Accounts)]
pub struct PlaceLimitOrderCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(init, payer = user, space = 8 + 64)]
    pub order: Account<'info, LimitOrder>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteLimitOrderCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut, has_one = user)]
    pub order: Account<'info, LimitOrder>,
}

#[derive(Accounts)]
pub struct MultiTokenSwapCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user_token_a_account: Account<'info, TokenAccount>,  // Add user token account
    #[account(mut)]
    pub user_token_b_account: Account<'info, TokenAccount>,  // Add user token account for the other token
    #[account(mut)]
    pub pool_token_a_account: Account<'info, TokenAccount>,  // Add pool token account for the input token
    #[account(mut)]
    pub pool_token_b_account: Account<'info, TokenAccount>,  // Add pool token account for the output token
    pub token_program: Program<'info, Token>,
}


// Implement the helper functions for MultiTokenSwapCtx
impl<'info> MultiTokenSwapCtx<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_a_account.to_account_info(),  // Assuming `user_token_a_account` exists for swap
                to: self.pool_token_a_account.to_account_info(),  // Assuming `pool_token_a_account` exists for swap
                authority: self.user.to_account_info(),
            },
        )
    }
}

#[derive(Accounts)]
pub struct FlashSwapCtx<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    pub token_program: Program<'info, Token>,
}

// Implement the helper functions for FlashSwapCtx
impl<'info> FlashSwapCtx<'info> {
    fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user.to_account_info(),
                to: self.pool.to_account_info(),
                authority: self.user.to_account_info(),
            },
        )
    }
}

// Event Definitions
#[event]
pub struct AddLiquidityEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
}

#[event]
pub struct SimpleSwapEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub fee: u64,
}

#[event]
pub struct LimitOrderPlaced {
    pub user: Pubkey,
    pub amount_a: u64,
    pub target_price: u64,
    pub expiration: i64,
    pub partial_fill: bool,
}

#[event]
pub struct LimitOrderExecuted {
    pub user: Pubkey,
    pub amount_a: u64,
    pub target_price: u64,
}

#[event]
pub struct MultiTokenSwapEvent {
    pub user: Pubkey,
    pub src_token: Pubkey,
    pub dst_token: Pubkey,
    pub amount: u64,
    pub received: u64,
}

#[event]
pub struct FlashSwapEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub target_contract: Pubkey,
}

// Utility Functions
fn calculate_fee(amount: u64, fee_rate: u64) -> u64 {
    (amount * fee_rate) / 1000
}

fn get_swap_amount(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Result<u64> {
    let amount_in_with_fee = amount_in * 997;
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = (reserve_in * 1000) + amount_in_with_fee;
    Ok(numerator / denominator)
}

fn route_swap(amount: u64, reserve_a: u64, reserve_b: u64) -> Result<u64> {
    let amount_out = (amount * reserve_b) / (reserve_a + amount);
    Ok(amount_out)
}

// Account Data Structures
#[account]
pub struct SwapState {
    pub fee_rate: u64,
    pub paused: bool,
}

#[account]
pub struct LiquidityPool {
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
}

#[account]
pub struct LimitOrder {
    pub user: Pubkey,
    pub token_a_reserve: u64,
    pub target_price: u64,
    pub expiration: i64,
    pub partial_fill: bool,
}

// Error Codes
#[error_code]
pub enum ErrorCode {
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Order expired")]
    OrderExpired,
    #[msg("Price conditions not met")]
    PriceNotMet,
    #[msg("Flash swap failed, tokens not returned")]
    FlashSwapFailed,
}

