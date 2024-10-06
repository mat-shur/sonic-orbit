use anchor_lang::{prelude::*, system_program};
use solana_program::pubkey;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, mint_to, Mint, MintTo, Token, TokenAccount},
    metadata::{create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3, Metadata as Metaplex}
};


declare_id!("orbpLfjHANgBMthuUp3ZsGewMwpE6HN6JXk9dypZ11U");
const GAME_OWNER_PUBKEY: Pubkey = pubkey!("orbwa31L7BZ2bTTg9QgUPTxAB7KnFfeU8oT9b56XG7f");


#[derive(Accounts)]
#[instruction(params: InitTokenParams)]
pub struct InitToken<'info> {
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = params.decimals,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metaplex>,
}


#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub destination: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitTokenParams {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
}


#[program]
pub mod orbit_leaderboard {
    use super::*;

    pub fn init_token(ctx: Context<InitToken>, metadata: InitTokenParams) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        let token_data: DataV2 = DataV2 {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            &signer
        );

        create_metadata_accounts_v3(
            metadata_ctx,
            token_data,
            false,
            true,
            None,
        )?;

        Ok(())
    }

    pub fn mint_tokens(ctx: Context<MintTokens>, quantity: u64) -> Result<()> {
        let seeds = &["mint".as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                &signer,
            ),
            quantity,
        )?;

        Ok(())
    }

    pub fn initialize_leaderboard(ctx: Context<InitializeLeaderboard>) -> Result<()> {
        let leaderboard = &mut ctx.accounts.leaderboard;
        leaderboard.initialize()
    }

    pub fn new_player(ctx: Context<NewPlayer>, username: String) -> Result<()> {
        let leaderboard = &mut ctx.accounts.leaderboard;

        require!(
            username.len() <= Player::USERNAME_MAX_LEN,
            OrbitLeaderboardError::UsernameTooLong
        );

        require!(
            leaderboard.players.len() < Leaderboard::MAX_PLAYERS,
            OrbitLeaderboardError::MaxPlayersReached
        );

        let registration_fee = 150_000_000;
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.game_owner.to_account_info(),
                },
            ),
            registration_fee,
        )?;

        let new_player = Player {
            username,
            pubkey: ctx.accounts.player.key(),
            last_score: 0,
            has_active_try: false,
            upgrades: 0
        };

        leaderboard.add_player(new_player)
    }

    pub fn new_try(ctx: Context<NewTry>) -> Result<()> {
        let leaderboard = &mut ctx.accounts.leaderboard;
        let player = leaderboard.get_player_mut(&ctx.accounts.player.key())?;

        let try_fee = 50_000_000;
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.game_owner.to_account_info(),
                },
            ),
            try_fee,
        )?;

        player.has_active_try = true;

        Ok(())
    }

    pub fn write_result(ctx: Context<WriteResult>, score: u64) -> Result<()> {
        let leaderboard = &mut ctx.accounts.leaderboard;
        let player = leaderboard.get_player_mut(&ctx.accounts.player.key())?;

        require!(player.has_active_try, OrbitLeaderboardError::NoActiveTry);

        if player.last_score < score {
            player.last_score = score;
        }
        
        player.has_active_try = false;

        let tokens_to_mint = score * 10u64.pow(ctx.accounts.mint.decimals as u32);
        let mint_bump = ctx.bumps.mint;
        let seeds: &[&[u8]] = &[b"mint", &[mint_bump]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.mint.to_account_info(),
                },
                &[seeds],
            ),
            tokens_to_mint,
        )?;

        Ok(())
    }

    pub fn buy_upgrade(ctx: Context<BuyUpgrade>) -> Result<()> {
        let leaderboard = &mut ctx.accounts.leaderboard;
        let player = leaderboard.get_player_mut(&ctx.accounts.player.key())?;

        if player.upgrades >= 24 {
            return Err(OrbitLeaderboardError::MaxUpgradesReached.into());
        }

        let upgrade_cost = 10_000 * 10u64.pow(ctx.accounts.mint.decimals as u32);

        if ctx.accounts.user_token_account.amount < upgrade_cost {
            return Err(OrbitLeaderboardError::InsufficientTokens.into());
        }

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.game_owner_token_account.to_account_info(),
                    authority: ctx.accounts.player.to_account_info(),
                },
            ),
            upgrade_cost,
        )?;

        player.upgrades += 1;

        Ok(())
    }
}


#[derive(Accounts)]
pub struct InitializeLeaderboard<'info> {
    #[account(
        init_if_needed,
        payer = game_owner,
        space = Leaderboard::SIZE,
        seeds = [b"leaderboard", game_owner.key().as_ref()],
        bump
    )]
    pub leaderboard: Account<'info, Leaderboard>,
    #[account(
        mut,
        address = GAME_OWNER_PUBKEY,
        owner = system_program::ID
    )]
    pub game_owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct NewPlayer<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        address = GAME_OWNER_PUBKEY,
        owner = system_program::ID
    )]
    pub game_owner: UncheckedAccount<'info>,
    #[account(mut)]
    pub leaderboard: Account<'info, Leaderboard>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct NewTry<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        address = GAME_OWNER_PUBKEY,
        owner = system_program::ID
    )]
    pub game_owner: UncheckedAccount<'info>,
    #[account(mut)]
    pub leaderboard: Account<'info, Leaderboard>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct WriteResult<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut)]
    pub leaderboard: Account<'info, Leaderboard>,
    #[account(
        init_if_needed,
        payer = player,
        associated_token::mint = mint,
        associated_token::authority = player,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


#[account]
pub struct Leaderboard {
    pub players: Vec<Player>,
}


impl Leaderboard {
    pub const MAX_PLAYERS: usize = 100;
    pub const SIZE: usize = 8 + 4 + (Self::MAX_PLAYERS * Player::SIZE);

    pub fn initialize(&mut self) -> Result<()> {
        self.players = Vec::new();
        Ok(())
    }

    pub fn add_player(&mut self, player: Player) -> Result<()> {
        if self.players.iter().any(|p| p.pubkey == player.pubkey) {
            return Err(OrbitLeaderboardError::PlayerAlreadyRegistered.into());
        }

        self.players.push(player);
        Ok(())
    }

    pub fn get_player_mut(&mut self, pubkey: &Pubkey) -> Result<&mut Player> {
        self.players
            .iter_mut()
            .find(|p| p.pubkey == *pubkey)
            .ok_or(OrbitLeaderboardError::PlayerNotFound.into())
    }
}

#[derive(Accounts)]
pub struct BuyUpgrade<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = player,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = game_owner,
    )]
    pub game_owner_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"leaderboard", GAME_OWNER_PUBKEY.as_ref()],
        bump,
    )]
    pub leaderboard: Account<'info, Leaderboard>,
    
    #[account(address = GAME_OWNER_PUBKEY)]
    pub game_owner: UncheckedAccount<'info>,
    
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Player {
    pub username: String,
    pub pubkey: Pubkey,
    pub last_score: u64,
    pub has_active_try: bool,
    pub upgrades: u8,
}


impl Player {
    pub const USERNAME_MAX_LEN: usize = 32;
    pub const SIZE: usize = 4 + Self::USERNAME_MAX_LEN + 32 + 8 + 1 + 1;
}


#[error_code]
pub enum OrbitLeaderboardError {
    #[msg("Player not found")]
    PlayerNotFound,
    #[msg("Player already registered")]
    PlayerAlreadyRegistered,
    #[msg("Username is too long")]
    UsernameTooLong,
    #[msg("Maximum number of players reached")]
    MaxPlayersReached,
    #[msg("Player has no active try")]
    NoActiveTry,
    #[msg("Invalid bump provided")]
    InvalidBump,
    #[msg("Maximum number of upgrades reached")]
    MaxUpgradesReached,
    #[msg("Insufficient tokens to buy upgrade")]
    InsufficientTokens,
}