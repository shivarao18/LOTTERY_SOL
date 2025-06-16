use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface}
};
use switchboard_on_demand::accounts::RandomnessAccountData;
use anchor_spl::metadata::{
    Metadata,
    MetadataAccount,
    CreateMetadataAccountsV3,
    CreateMasterEditionV3,
    SignMetadata,
    SetAndVerifySizedCollectionItem,
    create_master_edition_v3,
    create_metadata_accounts_v3,
    sign_metadata,
    set_and_verify_sized_collection_item,
    mpl_token_metadata::types::{
            CollectionDetails,
            Creator, 
            DataV2,
        },
};


declare_id!("2RTh2Y4e2N421EbSnUYTKdGqDHJH7etxZb3VrWDMpNMY");

#[constant]
pub const NAME: &str = "Token Lottery Ticket #";
#[constant]
pub const URI: &str = "Token Lottery";
#[constant]
pub const SYMBOL: &str = "TICKET";


#[program]
pub mod token_lottery {

    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConifg>, start: u64, end: u64, price: u64) -> Result<()> {
        ctx.accounts.token_lottery.bump = ctx.bumps.token_lottery;
        ctx.accounts.token_lottery.lottery_start = start;
        ctx.accounts.token_lottery.lottery_end = end;
        ctx.accounts.token_lottery.price = price;
        ctx.accounts.token_lottery.authority = ctx.accounts.payer.key();
        ctx.accounts.token_lottery.randomness_account = Pubkey::default();

        ctx.accounts.token_lottery.ticket_num = 0;
        ctx.accounts.token_lottery.winner_chosen = false;
        Ok(())
    }

    pub fn initialize_lottery(ctx: Context<InitializeLottery>) -> Result<()> {
        
        /* First of all we are creating a collection_mint , to mint token to our account we need a authority to sign, but in our logic we declared it as a collection mint address itself , since its a pda  we are usign signer seeds
        

        
        */
        //  Here , I got confused between collection 
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"collection_mint".as_ref(),
            &[ctx.bumps.collection_mint],
        ]];

        msg!("Creating mint accounts");

        // We need to use the mint_to function , we are doing a CPI , so we need ot follow a structure . Let me jsut practice it 

        /* 
        mint_to(
            CpiContext::new_with_signer(
                @Doubt : why do we need to_account_info(), what is the difference if we dont use that ? 
                ctx.accounts.token_program.to_account_info(),
                MintTO {
                    mint : ctx.accounts.collection_mint.to_account-info(),
                    to : ctx.accounts.collection_token_account.to_account_info(),
                    authority : ctx.accounts.collection_mint.to_account-info(),
                },
                signer_seeds
                ),
                1,
        )?;
        */
        mint_to(
            CpiContext::new_with_signer(
                // need to add token_program to the Accounts structure
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.collection_mint.to_account_info(),
                    to: ctx.accounts.collection_token_account.to_account_info(),
                    authority: ctx.accounts.collection_mint.to_account_info(),
                },
                signer_seeds,
            ),
            1,
            // ? in rust what does it do, how is it different from unwrap , when is one preferred over other/
        )?;

        // so far everythign is good and well , but why do we need the jargon below 
        // let me address that bullshit later 

        msg!("Creating metadata accounts");
        create_metadata_accounts_v3(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata.to_account_info(),
                    mint: ctx.accounts.collection_mint.to_account_info(),
                    mint_authority: ctx.accounts.collection_mint.to_account_info(), // use pda mint address as mint authority
                    update_authority: ctx.accounts.collection_mint.to_account_info(), // use pda mint as update authority
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
                &signer_seeds,
            ),
            DataV2 {
                name: NAME.to_string(),
                symbol: SYMBOL.to_string(),
                uri: URI.to_string(),
                seller_fee_basis_points: 0,
                creators: Some(vec![Creator {
                    address: ctx.accounts.collection_mint.key(),
                    verified: false,
                    share: 100,
                }]),
                collection: None,
                uses: None,
            },
            true,
            true,
            Some(CollectionDetails::V1 { size: 0 }), // set as collection nft
        )?;

        msg!("Creating Master edition accounts");
        create_master_edition_v3(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMasterEditionV3 {
                    payer: ctx.accounts.payer.to_account_info(),
                    mint: ctx.accounts.collection_mint.to_account_info(),
                    edition: ctx.accounts.master_edition.to_account_info(),
                    mint_authority: ctx.accounts.collection_mint.to_account_info(),
                    update_authority: ctx.accounts.collection_mint.to_account_info(),
                    metadata: ctx.accounts.metadata.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
                &signer_seeds,
            ),
            Some(0),
        )?;

        msg!("verifying collection");
        sign_metadata(CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            SignMetadata {
                creator: ctx.accounts.collection_mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
            },
            &signer_seeds,
        ))?;


        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        let clock = Clock::get()?;
        let ticket_name = NAME.to_owned() + ctx.accounts.token_lottery.ticket_num.to_string().as_str();
        
        if clock.slot < ctx.accounts.token_lottery.lottery_start || 
                clock.slot > ctx.accounts.token_lottery.lottery_end {
            return Err(ErrorCode::LotteryNotOpen.into());
        }

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.token_lottery.to_account_info(),
                },
            ),
            ctx.accounts.token_lottery.price,
        )?;

        ctx.accounts.token_lottery.lottery_pot_amount += ctx.accounts.token_lottery.price;

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"collection_mint".as_ref(),
            &[ctx.bumps.collection_mint],
        ]];

        // Mint Ticket
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.ticket_mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.collection_mint.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        create_metadata_accounts_v3(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata.to_account_info(),
                    mint: ctx.accounts.ticket_mint.to_account_info(),
                    mint_authority: ctx.accounts.collection_mint.to_account_info(),
                    update_authority: ctx.accounts.collection_mint.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
                &signer_seeds,
            ),
            DataV2 {
                name: ticket_name,
                symbol: SYMBOL.to_string(),
                uri: URI.to_string(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            true,
            true,
            None,
        )?;

        create_master_edition_v3(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMasterEditionV3 {
                    payer: ctx.accounts.payer.to_account_info(),
                    mint: ctx.accounts.ticket_mint.to_account_info(),
                    edition: ctx.accounts.master_edition.to_account_info(),
                    mint_authority: ctx.accounts.collection_mint.to_account_info(),
                    update_authority: ctx.accounts.collection_mint.to_account_info(),
                    metadata: ctx.accounts.metadata.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
                &signer_seeds,
            ),
            Some(0),
        )?;

        // verify nft as part of collection
        set_and_verify_sized_collection_item(
            CpiContext::new_with_signer(
                ctx.accounts.token_metadata_program.to_account_info(),
                SetAndVerifySizedCollectionItem {
                    metadata: ctx.accounts.metadata.to_account_info(),
                    collection_authority: ctx.accounts.collection_mint.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    update_authority: ctx.accounts.collection_mint.to_account_info(),
                    collection_mint: ctx.accounts.collection_mint.to_account_info(),
                    collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
                    collection_master_edition: ctx
                        .accounts
                        .collection_master_edition
                        .to_account_info(),
                },
                &signer_seeds,
            ),
            None,
        )?;

        ctx.accounts.token_lottery.ticket_num += 1;

        Ok(())
    }

    pub fn commit_a_winner(ctx: Context<CommitWinner>) -> Result<()> {
        let clock = Clock::get()?;
        let token_lottery = &mut ctx.accounts.token_lottery;
        if ctx.accounts.payer.key() != token_lottery.authority {
            return Err(ErrorCode::NotAuthorized.into());
        }

        let randomness_data = RandomnessAccountData::parse(ctx.accounts.randomness_account_data.data.borrow()).unwrap();

        if randomness_data.seed_slot != clock.slot - 1 {
            return Err(ErrorCode::RandomnessAlreadyRevealed.into());
        }

        token_lottery.randomness_account = ctx.accounts.randomness_account_data.key();

        Ok(())
    }

    pub fn choose_a_winner(ctx: Context<ChooseWinner>) -> Result<()> {
        let clock = Clock::get()?;
        let token_lottery = &mut ctx.accounts.token_lottery;

        if ctx.accounts.randomness_account_data.key() != token_lottery.randomness_account {
            return Err(ErrorCode::IncorrectRandomnessAccount.into());
        }
        if ctx.accounts.payer.key() != token_lottery.authority {
            return Err(ErrorCode::NotAuthorized.into());
        }
        if clock.slot < token_lottery.lottery_end {
            msg!("Current slot: {}", clock.slot);
            msg!("End slot: {}", token_lottery.lottery_end);
            return Err(ErrorCode::LotteryNotCompleted.into());
        }
        require!(token_lottery.winner_chosen == false, ErrorCode::WinnerChosen);

        let randomness_data = 
            RandomnessAccountData::parse(ctx.accounts.randomness_account_data.data.borrow()).unwrap();
        let revealed_random_value = randomness_data.get_value(&clock)
            .map_err(|_| ErrorCode::RandomnessNotResolved)?;

        msg!("Randomness result: {}", revealed_random_value[0]);
        msg!("Ticket num: {}", token_lottery.ticket_num);

        let randomness_result = 
            revealed_random_value[0] as u64 % token_lottery.ticket_num;

        msg!("Winner: {}", randomness_result);

        token_lottery.winner = randomness_result;
        token_lottery.winner_chosen = true;

        Ok(())
    }

    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        // Check if winner has been chosen
        msg!("Winner chosen: {}", ctx.accounts.token_lottery.winner_chosen);
        require!(ctx.accounts.token_lottery.winner_chosen, ErrorCode::WinnerNotChosen);
        
        // Check if token is a part of the collection
        require!(ctx.accounts.metadata.collection.as_ref().unwrap().verified, ErrorCode::NotVerifiedTicket);
        require!(ctx.accounts.metadata.collection.as_ref().unwrap().key == ctx.accounts.collection_mint.key(), ErrorCode::IncorrectTicket);

        let ticket_name = NAME.to_owned() + &ctx.accounts.token_lottery.winner.to_string();
        let metadata_name = ctx.accounts.metadata.name.replace("\u{0}", "");


        msg!("Ticket name: {}", ticket_name);
        msg!("Metdata name: {}", metadata_name);

        // Check if the winner has the winning ticket
        require!(metadata_name == ticket_name, ErrorCode::IncorrectTicket);
        require!(ctx.accounts.destination.amount > 0, ErrorCode::IncorrectTicket);

        **ctx.accounts.token_lottery.to_account_info().try_borrow_mut_lamports()? -= ctx.accounts.token_lottery.lottery_pot_amount;
        **ctx.accounts.payer.try_borrow_mut_lamports()? += ctx.accounts.token_lottery.lottery_pot_amount;

        ctx.accounts.token_lottery.lottery_pot_amount = 0;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump,
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(
        mut,
        seeds = [b"collection_mint".as_ref()],
        bump,
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [token_lottery.winner.to_le_bytes().as_ref()],
        bump,
    )]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [b"metadata", token_metadata_program.key().as_ref(), ticket_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata: Account<'info, MetadataAccount>,

    #[account(
        associated_token::mint = ticket_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), collection_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub collection_metadata: Account<'info, MetadataAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
}

#[derive(Accounts)]
pub struct CommitWinner<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump,
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    /// CHECK: The account's data is validated manually within the handler.
    pub randomness_account_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ChooseWinner<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump,
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    /// CHECK: The account's data is validated manually within the handler.
    pub randomness_account_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token_lottery".as_ref()],
        bump = token_lottery.bump
    )]
    pub token_lottery: Account<'info, TokenLottery>,

    #[account(
        init,
        payer = payer,
        seeds = [token_lottery.ticket_num.to_le_bytes().as_ref()],
        bump,
        mint::decimals = 0,
        mint::authority = collection_mint,
        mint::freeze_authority = collection_mint,
        mint::token_program = token_program
    )]
    pub ticket_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = ticket_mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), 
        ticket_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub metadata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), 
            ticket_mint.key().as_ref(), b"edition"],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub master_edition: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), collection_mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_metadata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), 
            collection_mint.key().as_ref(), b"edition"],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_master_edition: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"collection_mint".as_ref()],
        bump,
    )]
    pub collection_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeConifg<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + TokenLottery::INIT_SPACE,
        // Challenge: Make this be able to run more than 1 lottery at a time
        seeds = [b"token_lottery".as_ref()],
        bump
    )]
    pub token_lottery: Box<Account<'info, TokenLottery>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeLottery<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = collection_mint,
        mint::freeze_authority = collection_mint,
        seeds = [b"collection_mint".as_ref()],
        bump,
    )]
    pub collection_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: This account will be initialized by the metaplex program
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,

    /// CHECK: This account will be initialized by the metaplex program
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        seeds = [b"collection_token_account".as_ref()],
        bump,
        token::mint = collection_mint,
        token::authority = collection_token_account
    )]
    pub collection_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
#[derive(InitSpace)]
pub struct TokenLottery {
    pub bump: u8,
    pub winner: u64,
    pub winner_chosen: bool,
    pub lottery_start: u64,
    pub lottery_end: u64,
    // Is it good practice to store SOL on an account used for something else?
    pub lottery_pot_amount: u64,
    pub ticket_num: u64,
    pub price: u64,
    pub randomness_account: Pubkey,
    pub authority: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Incorrect randomness account")]
    IncorrectRandomnessAccount,
    #[msg("Lottery not completed")]
    LotteryNotCompleted,
    #[msg("Lottery is not open")]
    LotteryNotOpen,
    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Randomness already revealed")]
    RandomnessAlreadyRevealed,
    #[msg("Randomness not resolved")]
    RandomnessNotResolved,
    #[msg("Winner not chosen")]
    WinnerNotChosen,
    #[msg("Winner already chosen")]
    WinnerChosen,
    #[msg("Ticket is not verified")]
    NotVerifiedTicket,
    #[msg("Incorrect ticket")]
    IncorrectTicket,
}



///// explaination for the instructions 


////////////////

//First instruction is straightforward and doesnt need much explaination , only problem is intuitively knowing all the fields , but you can alwasy comeback and add fields necessary, same for the insane no. of imports 

///////////////
// Second Instruction : Lets dive into the core idea first 
/*
The initialize_lottery Instruction
The function executes a series of Cross-Program Invocations (CPIs) to the Token and Token Metadata programs to set up this collection NFT.

Define Signer Seeds:

let signer_seeds...: The program defines seeds for a Program Derived Address (PDA). This PDA (collection_mint) will act as the authority for creating the collection NFT and, later, for minting the individual tickets. This allows your program to sign for transactions without needing a private key.
Mint the Collection Token:

mint_to(...): A CPI is made to the Token Program. It mints exactly one token to the collection_token_account. The authority for this minting is the collection_mint PDA, which is why the signer_seeds are required.
Create Metadata:

create_metadata_accounts_v3(...): A CPI is made to the Metaplex Token Metadata program. This attaches data like the name (Token Lottery Ticket #), symbol (TICKET), and a URI to the token we just minted, turning it into a proper NFT. It also sets the collection details, marking it as a collection parent with a size of 0 (which can be expanded later).
Create Master Edition:

create_master_edition_v3(...): Another CPI to the Metaplex program. This step makes the NFT a "Master Edition." This is crucial because only a Master Edition can serve as the parent for other NFTs (the lottery tickets) in a collection.
Verify Creator:

sign_metadata(...): The final step is for the creator (collection_mint PDA) to "sign" the metadata. This action verifies the creator, which is a required step for a valid collection NFT.


An important difference : 
1. Are collection_mint and the "Collection NFT" the same?
No, they are not the same thing, but they are fundamentally linked.

Think of it with an analogy:

The collection_mint is like a unique master blueprint for a car. It defines the car's specifications (e.g., "there can only ever be one of this model") and who has the authority to build it. This blueprint is just data; it's a record on the blockchain.
The "Collection NFT" is the actual, physical car built from that blueprint. It's the tangible object that can be held in a garage (a Token Account).
In Solana terms, an "NFT" isn't a single thing. It's a combination of at least three accounts working together:

A Mint Account (collection_mint): The "blueprint" that defines the token.
A Token Account (collection_token_account): The "wallet" or "vault" that holds an instance of the token.
A Metadata Account (metadata): The account that attaches the NFT's properties (name, image, etc.) to the Mint.
So, to answer your other question: the collection_token_account is the account that holds the Collection NFT, but it isn't the NFT itself. The NFT is the whole package.

2. What makes the collection_mint unique?
This is where the magic of Program Derived Addresses (PDAs) comes in.

In your InitializeLottery struct, the collection_mint is defined with these seeds:
seeds = [b"collection_mint".as_ref()]

A PDA is a unique address derived from a set of "seeds" and a program ID.

The seed is "collection_mint".
The program ID is your lottery program's address (2RTh...pNMY).
There can only ever be one valid address generated from this specific combination of seeds and program ID. No matter how many times you call initialize_lottery, it will always derive the exact same address. Since the init constraint will fail if the account at that address already exists, you physically cannot create a second collection_mint for this program.

This guarantees your lottery program has only one unique collection "blueprint."

3. Can we mint multiple "Collection NFTs"?
This is a great observation. Based on the collection_mint alone, you could theoretically mint more than one token. However, the code prevents this in two key ways:

Minting Only One:
In the initialize_lottery function, the code explicitly mints a supply of exactly one:

Rust

mint_to(
    ...,
    1, // Minting just one token
)?;
Creating a Master Edition:
The most important step is this one:

Rust

create_master_edition_v3( ... )?;
Creating a "Master Edition" for a mint does two things:

It signifies that this NFT is the "master copy."
It disables the minting authority. After a Master Edition is created, you can no longer use the collection_mint to create any more tokens. The supply is permanently locked at 1.
This is precisely what makes the "Collection NFT" a unique, non-fungible item. You create the blueprint (collection_mint), use it once to build the car ("Collection NFT"), and then the create_master_edition_v3 call effectively destroys the blueprint so it can never be used again

//////////////

// Third Instruction : so far, everything is done by the creator of the lottery 
@ Doubt : there is no relation between the first instruction and second instruction, let me check that later 

let some randon guy buy a ticket , what accounts do we need for this 

obviously the one of the guy  who pays , 
the collection_mint and the address that holds it so that it can mint more tokens that belong to this collection
this guy probaly needs a account to hold the token 
also what detials deos this guys token need ot hold 

//instructions 

what are the checks 

how do can we the guy a ticket 




