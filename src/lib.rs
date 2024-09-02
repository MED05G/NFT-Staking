use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token};
use anchor_spl::token::{Mint, TokenAccount};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

declare_id!("9Jg8DdfXGb96bbgW2aKeRDCXtXNFDm9LcaUBA3SvTiSz");

mod admin {
    anchor_lang::declare_id!("Bw3TEbFp65WtpjhqpM2fggtgvx8LJKo3kqTyDeaHH3EB");
}

#[program]
pub mod nftpledge {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    /// Pledge NFT
    pub fn pledge_nft(ctx: Context<PledgeNft>, timestamp: i64, days: i64) -> Result<()> {
        // get random value
        let solt_hash_account = ctx.accounts.slot_hashes.to_account_info();
        let data = solt_hash_account.data.borrow();
        let slot_hashes = bincode::deserialize::<SlotHashes>(&data).unwrap();
        let slot_hash = slot_hashes.first().unwrap();
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(slot_hash.0);
        hasher.write_i64(days);
        hasher.write_i64(Clock::get().unwrap().unix_timestamp);
        hasher.write(&slot_hash.1.to_bytes()[..]);
        let hash_num = hasher.finish();
        let random_value = hash_num % (500000 - 100000 + 1) + 100000; // 9951 is the range (10000 - 50 + 1)
        msg!("Random value generated: {}", random_value);

        // update global
        let global = &mut ctx.accounts.global;
        global.nfts += 1;
        global.tokens += random_value;

        // update saleinfo
        let saleinfo = &mut ctx.accounts.saleinfo;
        saleinfo.owner = ctx.accounts.user.to_account_info().key();
        saleinfo.mint = ctx.accounts.nft_mint.key();
        saleinfo.tokens = random_value;w
        saleinfo.timestamp = timestamp;
        saleinfo.unlock_timestamp = Clock::get()?.unix_timestamp + (days as i64) * 24 * 3600;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.plat_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            1,
        )?;

        Ok(())
    }

    /// Unpledge NFT
    pub fn unpledge_nft(ctx: Context<PledgeNft>, timestamp: i64) -> Result<()> {
        let saleinfo = &mut ctx.accounts.saleinfo;
        if saleinfo.owner != ctx.accounts.user.to_account_info().key() {
            return Err(ErrorCode::AuthorityNotMatch.into());
        }

        if Clock::get()?.unix_timestamp < saleinfo.unlock_timestamp {
            return Err(ErrorCode::TimeNotReach.into());
        }

        // close account
        ctx.accounts
            .saleinfo
            .close(ctx.accounts.user.to_account_info())?;

        let binding = ctx.accounts.nft_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"plat_nft",
            binding.as_ref(),
            &[ctx.bumps.plat_token_account],
        ]];
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.plat_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.plat_token_account.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        Ok(())
    }
}

#[derive(Debug)]
#[account]
pub struct Global {
    pub tokens: u64,
    pub nfts: u64,
}
impl Global {
    const LEN: usize = 8 + 8 + 8;
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init, payer = user, seeds = [b"global"], bump, space = Global::LEN)]
    pub global: Account<'info, Global>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Debug)]
#[account]
pub struct NftSale {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub tokens: u64,
    pub timestamp: i64,
    pub unlock_timestamp: i64,
}
impl NftSale {
    const LEN: usize = 8 + 32 * 2 + 8 * 3;
}

#[derive(Accounts)]
#[instruction(timestamp: i64)]
pub struct PledgeNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"global"], bump)]
    pub global: Box<Account<'info, Global>>,
    #[account(init_if_needed, payer = user, seeds = [&timestamp.to_le_bytes()], bump, space = NftSale::LEN)]
    pub saleinfo: Box<Account<'info, NftSale>>,
    pub nft_mint: Account<'info, Mint>,
    #[account(mut, token::mint = nft_mint, token::authority = user)]
    pub user_token_account: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, payer = user, seeds=[b"plat_nft", nft_mint.key().as_ref()], bump, token::mint = nft_mint, token::authority = plat_token_account)]
    pub plat_token_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    /// CHECK
    #[account(address =  anchor_lang::solana_program::sysvar::slot_hashes::id())]
    slot_hashes: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Authority not match")]
    AuthorityNotMatch,
    #[msg("Time not reach")]
    TimeNotReach,
}
