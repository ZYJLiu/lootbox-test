use anchor_lang::prelude::*;

pub mod actions;
pub use actions::*;

pub use anchor_lang::solana_program::clock;
pub use anchor_spl::{
    associated_token::{get_associated_token_address, AssociatedToken},
    token::{self, Mint, MintTo, Token, TokenAccount},
};
pub use switchboard_v2::{
    OracleQueueAccountData, PermissionAccountData, SbState, VrfAccountData, VrfRequestRandomness,
};

declare_id!("ExH9v81oi6CZuGhDAdtuCf6R4XTaLvq9h9tr7bHhz6QA");

#[program]
pub mod lootbox {
    use super::*;

    #[access_control(ctx.accounts.validate(&ctx))]
    pub fn init_lootbox(mut ctx: Context<InitLootbox>) -> Result<()> {
        InitLootbox::actuate(&mut ctx)
    }

    #[access_control(ctx.accounts.validate(&ctx, &params))]
    pub fn init_user(mut ctx: Context<InitUser>, params: InitUserParams) -> Result<()> {
        InitUser::actuate(&mut ctx, &params)
    }

    #[access_control(ctx.accounts.validate(&ctx))]
    pub fn request_randomness(ctx: Context<RequestRandomness>) -> Result<()> {
        RequestRandomness::actuate(&ctx)
    }

    #[access_control(ctx.accounts.validate(&ctx))]
    pub fn consume_randomness(ctx: Context<ConsumeRandomness>) -> Result<()> {
        ConsumeRandomness::actuate(&ctx)
    }

    #[access_control(ctx.accounts.validate(&ctx))]
    pub fn mint_reward(mut ctx: Context<MintReward>) -> Result<()> {
        MintReward::actuate(&mut ctx)
    }
}

const LOOTBOX_SEED: &str = "LOOTBOX";
const MINT_AUTH_SEED: &str = "MINT_AUTH";

#[repr(packed)]
#[account(zero_copy)]
#[derive(Default)]
pub struct UserState {
    pub bump: u8,
    pub switchboard_state_bump: u8,
    pub vrf_permission_bump: u8,
    pub result_buffer: [u8; 32],
    pub result: u128,
    pub vrf: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub redeemable: bool,
}

#[account]
#[derive(Default, PartialEq)]
pub struct Lootbox {
    pub mint_one: Pubkey,
    pub mint_two: Pubkey,
    pub mint_three: Pubkey,
}

#[error_code]
#[derive(Eq, PartialEq)]
pub enum LootboxErrorCode {
    #[msg("Switchboard VRF Account's authority should be set to the client's state pubkey")]
    InvalidVrfAuthorityError,
    #[msg("Invalid VRF account provided.")]
    InvalidVrfAccount,
    #[msg("Already Redeemed")]
    AlreadyRedeemed,
}
