use crate::*;

#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(
        mut,
        seeds = [
            payer.key().as_ref(),
        ],
        bump = state.load()?.bump,
        has_one = vrf @ LootboxErrorCode::InvalidVrfAccount
    )]
    pub state: AccountLoader<'info, UserState>,

    // SWITCHBOARD ACCOUNTS
    #[account(mut,
        has_one = escrow
    )]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    #[account(mut,
        has_one = data_buffer
    )]
    pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,
    /// CHECK:
    #[account(mut,
        constraint =
            oracle_queue.load()?.authority == queue_authority.key()
    )]
    pub queue_authority: UncheckedAccount<'info>,
    /// CHECK
    #[account(mut)]
    pub data_buffer: AccountInfo<'info>,
    #[account(mut)]
    pub permission: AccountLoader<'info, PermissionAccountData>,
    #[account(mut,
        constraint =
            escrow.owner == program_state.key()
            && escrow.mint == program_state.load()?.token_mint
    )]
    pub escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    pub program_state: AccountLoader<'info, SbState>,
    /// CHECK:
    #[account(
        address = *vrf.to_account_info().owner,
        constraint = switchboard_program.executable == true
    )]
    pub switchboard_program: AccountInfo<'info>,

    // PAYER ACCOUNTS
    #[account(mut,
        constraint =
            payer_wallet.owner == payer.key()
            && escrow.mint == program_state.load()?.token_mint
    )]
    pub payer_wallet: Account<'info, TokenAccount>,
    /// CHECK:
    #[account(signer)]
    pub payer: AccountInfo<'info>,
    // SYSTEM ACCOUNTS
    /// CHECK:
    #[account(address = solana_program::sysvar::recent_blockhashes::ID)]
    pub recent_blockhashes: AccountInfo<'info>,

    // Stake Reward Mint
    #[account(mut)]
    pub stake_mint: Account<'info, Mint>,
    // Stake Reward Account
    #[account(mut,
        token::authority = payer)]
    pub stake_token_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

impl RequestRandomness<'_> {
    pub fn validate(&self, _ctx: &Context<Self>) -> Result<()> {
        Ok(())
    }
    pub fn actuate(ctx: &Context<Self>) -> Result<()> {
        let state = ctx.accounts.state.load()?;
        let bump = state.bump.clone();
        let switchboard_state_bump = state.switchboard_state_bump;
        let vrf_permission_bump = state.vrf_permission_bump;
        drop(state);

        let switchboard_program = ctx.accounts.switchboard_program.to_account_info();

        let vrf_request_randomness = VrfRequestRandomness {
            authority: ctx.accounts.state.to_account_info(),
            vrf: ctx.accounts.vrf.to_account_info(),
            oracle_queue: ctx.accounts.oracle_queue.to_account_info(),
            queue_authority: ctx.accounts.queue_authority.to_account_info(),
            data_buffer: ctx.accounts.data_buffer.to_account_info(),
            permission: ctx.accounts.permission.to_account_info(),
            escrow: ctx.accounts.escrow.clone(),
            payer_wallet: ctx.accounts.payer_wallet.clone(),
            payer_authority: ctx.accounts.payer.to_account_info(),
            recent_blockhashes: ctx.accounts.recent_blockhashes.to_account_info(),
            program_state: ctx.accounts.program_state.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        let payer = ctx.accounts.payer.key();
        let state_seeds: &[&[&[u8]]] = &[&[payer.as_ref(), &[bump]]];

        msg!("requesting randomness");
        vrf_request_randomness.invoke_signed(
            switchboard_program,
            switchboard_state_bump,
            vrf_permission_bump,
            state_seeds,
        )?;

        let mut state = ctx.accounts.state.load_mut()?;
        state.result = 0;
        state.redeemable = true;

        msg!("randomness requested successfully");

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.stake_mint.to_account_info(),
                from: ctx.accounts.stake_token_account.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        // token::burn(cpi_ctx, 100)?;
        token::burn(
            cpi_ctx,
            (10 as u64)
                .checked_mul(
                    (10 as u64)
                        .checked_pow(*&ctx.accounts.stake_mint.decimals as u32)
                        .unwrap(),
                )
                .unwrap(),
        )?;

        Ok(())
    }
}
