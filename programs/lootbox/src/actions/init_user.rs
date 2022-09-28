use crate::*;

#[derive(Accounts)]
#[instruction(params: InitUserParams)]
pub struct InitUser<'info> {
    #[account(
        init,
        seeds = [
            payer.key().as_ref(),
            vrf.key().as_ref()
        ],
        payer = payer,
        space = 8 + std::mem::size_of::<UserState>(),
        bump,
    )]
    pub state: AccountLoader<'info, UserState>,
    #[account(
        constraint = vrf.load()?.authority == state.key() @ LootboxErrorCode::InvalidVrfAuthorityError
    )]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitUserParams {
    pub switchboard_state_bump: u8,
    pub vrf_permission_bump: u8,
}

impl InitUser<'_> {
    pub fn validate(&self, _ctx: &Context<Self>, _params: &InitUserParams) -> Result<()> {
        Ok(())
    }

    pub fn actuate(ctx: &Context<Self>, params: &InitUserParams) -> Result<()> {
        msg!("init_client actuate");

        let mut state = ctx.accounts.state.load_init()?;
        *state = UserState::default();
        state.bump = ctx.bumps.get("state").unwrap().clone();
        state.switchboard_state_bump = params.switchboard_state_bump;
        state.vrf_permission_bump = params.vrf_permission_bump;
        state.vrf = ctx.accounts.vrf.key();

        Ok(())
    }
}
