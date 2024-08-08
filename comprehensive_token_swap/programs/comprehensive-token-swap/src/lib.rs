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
        // Extract necessary values from the context
        let token_program_info = ctx.accounts.token_program.to_account_info();
        let user_token_a_account = ctx.accounts.user_token_a_account.to_account_info();
        let pool_token_a_account = ctx.accounts.pool_token_a_account.to_account_info();
        let user_token_b_account = ctx.accounts.user_token_b_account.to_account_info();
        let pool_token_b_account = ctx.accounts.pool_token_b_account.to_account_info();

        // Create transfer contexts before mutably borrowing ctx.accounts
        let transfer_ctx_to_pool = CpiContext::new(
            token_program_info.clone(),
            Transfer {
                from: user_token_a_account.clone(),
                to: pool_token_a_account.clone(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        let transfer_ctx_to_user = CpiContext::new(
            token_program_info.clone(),
            Transfer {
                from: pool_token_b_account.clone(),
                to: user_token_b_account.clone(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        let transfer_ctx_fee = CpiContext::new(
            token_program_info,
            Transfer {
                from: pool_token_b_account,
                to: user_token_b_account,
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        // Mutably borrow ctx.accounts
        let pool = &mut ctx.accounts.pool;
        let swap = &mut ctx.accounts.swap;

        let amount_b = get_swap_amount(amount_a, pool.token_a_reserve, pool.token_b_reserve)?;
        let fee = calculate_fee(amount_b, swap.fee_rate);

        require!(amount_b >= min_amount_b, ErrorCode::SlippageExceeded);

        // Perform token transfers
        token::transfer(transfer_ctx_to_pool, amount_a)?;
        token::transfer(transfer_ctx_to_user, amount_b - fee)?;
        token::transfer(transfer_ctx_fee, fee)?;

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

    // Add other methods for limit orders, multi-token swaps, flash swaps, etc.
}

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

// Event definitions
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

// Utility functions
fn calculate_fee(amount: u64, fee_rate: u64) -> u64 {
    (amount * fee_rate) / 1000
}

fn get_swap_amount(amount_in: u64, reserve_in: u64, reserve_out: u64) -> Result<u64> {
    let amount_in_with_fee = amount_in * 997;
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = (reserve_in * 1000) + amount_in_with_fee;
    Ok(numerator / denominator)
}

#[error_code]
pub enum ErrorCode {
    #[msg("Slippage exceeded")]
    SlippageExceeded,
}
