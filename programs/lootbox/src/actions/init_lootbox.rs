use crate::*;

#[derive(Accounts)]
pub struct InitLootbox<'info> {
    #[account(
        init,
        seeds = [
            LOOTBOX_SEED.as_bytes(),
        ],
        payer = payer,
        space = 8 + std::mem::size_of::<Lootbox>(),
        bump,
    )]
    pub lootbox: Account<'info, Lootbox>,
    #[account(
       mint::authority = mint_auth
    )]
    pub mint_one: Account<'info, Mint>,
    #[account(
       mint::authority = mint_auth
    )]
    pub mint_two: Account<'info, Mint>,
    #[account(
       mint::authority = mint_auth
    )]
    pub mint_three: Account<'info, Mint>,
    #[account(
        seeds = [
            MINT_AUTH_SEED.as_bytes(),
        ],
        bump,
    )]
    /// CHECK: mint authority PDA
    pub mint_auth: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl InitLootbox<'_> {
    pub fn validate(&self, _ctx: &Context<Self>) -> Result<()> {
        Ok(())
    }

    pub fn actuate(ctx: &mut Context<Self>) -> Result<()> {
        msg!("Initialize Lootbox");

        ctx.accounts.lootbox.mint_one = ctx.accounts.mint_one.key();
        ctx.accounts.lootbox.mint_two = ctx.accounts.mint_two.key();
        ctx.accounts.lootbox.mint_three = ctx.accounts.mint_three.key();

        msg!("Mint One: {:?}", ctx.accounts.mint_one.key());
        msg!("Mint Two: {:?}", ctx.accounts.mint_two.key());
        msg!("Mint Three: {:?}", ctx.accounts.mint_three.key());
        Ok(())
    }
}
