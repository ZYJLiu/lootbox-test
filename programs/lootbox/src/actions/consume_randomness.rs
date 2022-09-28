use crate::*;

#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    #[account(
        mut,
        seeds = [
            payer.key().as_ref(),
            vrf.key().as_ref(),
        ],
        bump = state.load()?.bump,
        has_one = vrf @ LootboxErrorCode::InvalidVrfAccount
    )]
    pub state: AccountLoader<'info, UserState>,
    pub vrf: AccountLoader<'info, VrfAccountData>,
    pub lootbox: Account<'info, Lootbox>,
    /// CHECK:
    pub payer: AccountInfo<'info>,
}

impl ConsumeRandomness<'_> {
    pub fn validate(&self, _ctx: &Context<Self>) -> Result<()> {
        Ok(())
    }

    pub fn actuate(ctx: &Context<Self>) -> Result<()> {
        let vrf = ctx.accounts.vrf.load()?;
        let result_buffer = vrf.get_result()?;
        if result_buffer == [0u8; 32] {
            msg!("vrf buffer empty");
            return Ok(());
        }

        let state = &mut ctx.accounts.state.load_mut()?;

        // maximum value to convert randomness buffer
        let max_result = 100;
        if result_buffer == state.result_buffer {
            msg!("result_buffer unchanged");
            return Ok(());
        }

        msg!("Result buffer is {:?}", result_buffer);
        let value: &[u128] = bytemuck::cast_slice(&result_buffer[..]);
        msg!("u128 buffer {:?}", value);
        let result = value[0] % max_result as u128 + 1;
        msg!("Current VRF Value [1 - {}) = {}!", max_result, result);

        if state.result != result {
            state.result_buffer = result_buffer;
            state.result = result;
        }

        let one = 1..33;
        if one.contains(&result) {
            msg!("Mint One: {:?}", ctx.accounts.lootbox.mint_one);
            let token_address = get_associated_token_address(
                &ctx.accounts.payer.key(),
                &ctx.accounts.lootbox.mint_one,
            );
            state.token_account = token_address;
            state.mint = ctx.accounts.lootbox.mint_one;
        }
        let two = 34..66;
        if two.contains(&result) {
            msg!("Mint Two: {:?}", ctx.accounts.lootbox.mint_two);
            let token_address = get_associated_token_address(
                &ctx.accounts.payer.key(),
                &ctx.accounts.lootbox.mint_two,
            );
            state.token_account = token_address;
            state.mint = ctx.accounts.lootbox.mint_two
        }
        let three = 67..100;
        if three.contains(&result) {
            msg!("Mint Three: {:?}", ctx.accounts.lootbox.mint_three);
            let token_address = get_associated_token_address(
                &ctx.accounts.payer.key(),
                &ctx.accounts.lootbox.mint_three,
            );
            state.token_account = token_address;
            state.mint = ctx.accounts.lootbox.mint_three
        }
        Ok(())
    }
}
