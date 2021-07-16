use crate::{dex, open_orders_authority, open_orders_init_authority};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::Accounts;
use serum_dex::instruction::*;
use serum_dex::state::OpenOrders;
use std::mem::size_of;

/// Per request context. Can be used to share data between middleware handlers.
pub struct Context<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub accounts: Vec<AccountInfo<'info>>,
    pub data: &'a mut &'a [u8],
    pub seeds: Vec<Vec<Vec<u8>>>,
}

impl<'a, 'info> Context<'a, 'info> {
    pub fn new(
        program_id: &'a Pubkey,
        accounts: Vec<AccountInfo<'info>>,
        data: &'a mut &'a [u8],
    ) -> Self {
        Self {
            program_id,
            accounts,
            data,
            seeds: Vec::new(),
        }
    }
}

/// Implementing this trait allows one to hook into requests to the Serum DEX
/// via a frontend proxy.
pub trait MarketMiddleware {
    fn init_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    fn new_order_v3(&self, _ctx: &mut Context, _ix: &NewOrderInstructionV3) -> ProgramResult {
        Ok(())
    }

    fn cancel_order_v2(&self, _ctx: &mut Context, _ix: &CancelOrderInstructionV2) -> ProgramResult {
        Ok(())
    }

    fn cancel_order_by_client_id_v2(&self, _ctx: &mut Context, _client_id: u64) -> ProgramResult {
        Ok(())
    }

    fn settle_funds(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    fn close_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }

    /// Called when the instruction data doesn't match any DEX instruction.
    fn fallback(&self, _ctx: &mut Context) -> ProgramResult {
        Ok(())
    }
}

/// Checks that the given open orders account signs the transaction and then
/// replaces it with the open orders account, which must be a PDA.
pub struct OpenOrdersPda;

impl OpenOrdersPda {
    fn prepare_pda<'info>(acc_info: &AccountInfo<'info>) -> AccountInfo<'info> {
        let mut acc_info = acc_info.clone();
        acc_info.is_signer = true;
        acc_info
    }
}

impl MarketMiddleware for OpenOrdersPda {
    /// Accounts:
    ///
    /// 0. Dex program.
    /// 1. System program.
    /// .. serum_dex::MarketInstruction::InitOpenOrders.
    fn init_open_orders<'a, 'info>(&self, ctx: &mut Context<'a, 'info>) -> ProgramResult {
        let market = &ctx.accounts[4];
        let user = &ctx.accounts[3];

        // Find canonical bump seeds.
        let (_, bump) = Pubkey::find_program_address(
            &[
                b"open-orders".as_ref(),
                market.key.as_ref(),
                user.key.as_ref(),
            ],
            ctx.program_id,
        );
        let (_, bump_init) = Pubkey::find_program_address(
            &[b"open-orders-init".as_ref(), ctx.accounts[4].key.as_ref()],
            ctx.program_id,
        );

        // Initialize PDA.
        let mut accounts = &ctx.accounts[..];
        InitAccount::try_accounts(ctx.program_id, &mut accounts, &[bump, bump_init])?;

        // Add signer to context.
        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key,
            bump = bump
        });
        ctx.seeds.push(open_orders_init_authority! {
            program = ctx.program_id,
            market = market.key,
            bump = bump_init
        });

        // Chop off the first two accounts needed for initializing the PDA.
        ctx.accounts = (&ctx.accounts[2..]).to_vec();

        // Set PDAs.
        ctx.accounts[1] = Self::prepare_pda(&ctx.accounts[0]);
        ctx.accounts[4].is_signer = true;

        Ok(())
    }

    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::NewOrderV3.
    fn new_order_v3(&self, ctx: &mut Context, _ix: &NewOrderInstructionV3) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[7];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[7] = Self::prepare_pda(&ctx.accounts[1]);

        Ok(())
    }

    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::CancelOrderV2.
    fn cancel_order_v2(&self, ctx: &mut Context, _ix: &CancelOrderInstructionV2) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[4];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[4] = Self::prepare_pda(&ctx.accounts[3]);

        Ok(())
    }

    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::CancelOrderByClientIdV2.
    fn cancel_order_by_client_id_v2(&self, ctx: &mut Context, _client_id: u64) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[4];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[4] = Self::prepare_pda(&ctx.accounts[3]);

        Ok(())
    }

    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::SettleFunds.
    fn settle_funds(&self, ctx: &mut Context) -> ProgramResult {
        let market = &ctx.accounts[0];
        let user = &ctx.accounts[2];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[2] = Self::prepare_pda(&ctx.accounts[1]);

        Ok(())
    }

    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::CloseOpenOrders.
    fn close_open_orders(&self, ctx: &mut Context) -> ProgramResult {
        let market = &ctx.accounts[3];
        let user = &ctx.accounts[1];
        if !user.is_signer {
            return Err(ErrorCode::UnauthorizedUser.into());
        }

        ctx.seeds.push(open_orders_authority! {
            program = ctx.program_id,
            market = market.key,
            authority = user.key
        });

        ctx.accounts[1] = Self::prepare_pda(&ctx.accounts[0]);

        Ok(())
    }
}

/// Logs each request.
pub struct Logger;
impl MarketMiddleware for Logger {
    fn init_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying open orders");
        Ok(())
    }

    fn new_order_v3(&self, _ctx: &mut Context, ix: &NewOrderInstructionV3) -> ProgramResult {
        msg!("proxying new order v3 {:?}", ix);
        Ok(())
    }

    fn cancel_order_v2(&self, _ctx: &mut Context, ix: &CancelOrderInstructionV2) -> ProgramResult {
        msg!("proxying cancel order v2 {:?}", ix);
        Ok(())
    }

    fn cancel_order_by_client_id_v2(&self, _ctx: &mut Context, client_id: u64) -> ProgramResult {
        msg!("proxying cancel order by client id v2 {:?}", client_id);
        Ok(())
    }

    fn settle_funds(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying cancel order by client id v2");
        Ok(())
    }

    fn close_open_orders(&self, _ctx: &mut Context) -> ProgramResult {
        msg!("proxying close open orders");
        Ok(())
    }
}

/// Enforces referal fees being sent to the configured address.
pub struct ReferralFees {
    referral: Pubkey,
}

impl ReferralFees {
    pub fn new(referral: Pubkey) -> Self {
        Self { referral }
    }
}

impl MarketMiddleware for ReferralFees {
    /// Accounts:
    ///
    /// .. serum_dex::MarketInstruction::SettleFunds.
    fn settle_funds(&self, ctx: &mut Context) -> ProgramResult {
        let referral = &ctx.accounts[9];
        let enabled = false;
        if enabled {
            require!(referral.key == &self.referral, ErrorCode::InvalidReferral);
        }
        Ok(())
    }
}

// Macros.

/// Returns the seeds used for a user's open orders account PDA.
#[macro_export]
macro_rules! open_orders_authority {
    (program = $program:expr, market = $market:expr, authority = $authority:expr, bump = $bump:expr) => {
        vec![
            b"open-orders".to_vec(),
            $market.as_ref().to_vec(),
            $authority.as_ref().to_vec(),
            vec![$bump],
        ]
    };
    (program = $program:expr, market = $market:expr, authority = $authority:expr) => {
        vec![
            b"open-orders".to_vec(),
            $market.as_ref().to_vec(),
            $authority.as_ref().to_vec(),
            vec![
                Pubkey::find_program_address(
                    &[
                        b"open-orders".as_ref(),
                        $market.as_ref(),
                        $authority.as_ref(),
                    ],
                    $program,
                )
                .1,
            ],
        ]
    };
}

/// Returns the seeds used for the open orders init authority.
/// This is the account that must sign to create a new open orders account on
/// the DEX market.
#[macro_export]
macro_rules! open_orders_init_authority {
    (program = $program:expr, market = $market:expr) => {
        vec![
            b"open-orders-init".to_vec(),
            $market.as_ref().to_vec(),
            vec![
                Pubkey::find_program_address(
                    &[b"open-orders-init".as_ref(), $market.as_ref()],
                    $program,
                )
                .1,
            ],
        ]
    };
    (program = $program:expr, market = $market:expr, bump = $bump:expr) => {
        vec![
            b"open-orders-init".to_vec(),
            $market.as_ref().to_vec(),
            vec![$bump],
        ]
    };
}

// Errors.

#[error]
pub enum ErrorCode {
    #[msg("Program ID does not match the Serum DEX")]
    InvalidDexPid,
    #[msg("Invalid instruction given")]
    InvalidInstruction,
    #[msg("Could not unpack the instruction")]
    CannotUnpack,
    #[msg("Invalid referral address given")]
    InvalidReferral,
    #[msg("The user didn't sign")]
    UnauthorizedUser,
    #[msg("Not enough accounts were provided")]
    NotEnoughAccounts,
}

#[derive(Accounts)]
#[instruction(bump: u8, bump_init: u8)]
pub struct InitAccount<'info> {
    #[account(address = dex::ID)]
    pub dex_program: AccountInfo<'info>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"open-orders", market.key.as_ref(), authority.key.as_ref()],
        bump = bump,
        payer = authority,
        owner = dex::ID,
        space = size_of::<OpenOrders>() + SERUM_PADDING,
    )]
    pub open_orders: AccountInfo<'info>,
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(seeds = [b"open-orders-init", market.key.as_ref(), &[bump_init]])]
    pub open_orders_init_authority: AccountInfo<'info>,
}

// Constants.

// Padding added to every serum account.
//
// b"serum".len() + b"padding".len().
const SERUM_PADDING: usize = 12;
