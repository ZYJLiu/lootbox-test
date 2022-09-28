use crate::*;

#[derive(Accounts)]
pub struct MintReward<'info> {
    #[account(mut)]
    pub state: AccountLoader<'info, UserState>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user
    )]
    pub token_account: Account<'info, TokenAccount>,
    /// CHECK: only used as a signing PDA
    #[account(seeds = [MINT_AUTH_SEED.as_bytes()], bump)]
    pub mint_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub user: Signer<'info>,
}

// mint randomly selected lootbox token
impl MintReward<'_> {
    pub fn validate(&self, _ctx: &Context<Self>) -> Result<()> {
        Ok(())
    }

    pub fn actuate(ctx: &mut Context<Self>) -> Result<()> {
        let mut client_state = ctx.accounts.state.load_mut()?;
        if client_state.redeemable != true {
            return Err(error!(LootboxErrorCode::AlreadyRedeemed));
        }
        client_state.redeemable = false;

        let seeds = &[
            MINT_AUTH_SEED.as_bytes(),
            &[*ctx.bumps.get("mint_authority").unwrap()],
        ];
        let signer = [&seeds[..]];

        msg!("Minting Reward");
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer);

        token::mint_to(cpi_ctx, 1)?;
        msg!("Token Minted");

        Ok(())
    }
}
