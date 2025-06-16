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
        //  Here , I got confused between collection_mint, collection_token_account and collection_nft 
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
            /* These three parameters are:
            true (is_mutable): This boolean indicates whether the metadata can be updated after creation. Setting it to true means the metadata is mutable and can be changed later by the update authority. If set to false, the metadata becomes immutable forever.
            true (update_authority_is_signer): This boolean indicates whether the update authority is required to be a signer for this transaction. Setting it to true means the update authority must sign the transaction when creating the metadata.
            Some(CollectionDetails::V1 { size: 0 }): This is an optional parameter that specifies collection details. When provided with CollectionDetails::V1 { size: 0 }, it marks this NFT as a collection parent (master collection NFT). The size: 0 indicates that this is a sized collection starting with 0 items, which will grow as individual NFTs are added to the collection.
            In your lottery contract context:
            The collection NFT (created in initialize_lottery) uses Some(CollectionDetails::V1 { size: 0 }) to mark it as a collection parent
            The individual ticket NFTs (created in buy_ticket) use None for this parameter since they are individual items, not collection parents
            This is how the Metaplex Token Metadata program distinguishes between collection parent NFTs and regular NFTs that can be part of collections.*/
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
            /* he Context: The "Reveal" Step
        First, remember where we are. This line is inside the choose_a_winner function. We have already:

        Committed to using a specific Switchboard randomness account in a previous transaction.
        Verified that the lottery is over and the account provided is the one we committed to.
        Now, we need to open the "sealed envelope" and get the actual number.

        Part 1: randomness_data.get_value(&clock)
        This is the core action, a function call to the Switchboard library.

        randomness_data: This is our organized, parsed struct containing the information from the Switchboard account.
        .get_value(): This is the function whose job is to retrieve the final, unpredictable random number.
        (&clock): This is the most important parameter. The random value stored on the Switchboard account is not a simple, plain number. It is often obfuscated or encrypted using a value derived from the slot it was created in (the seed_slot). The get_value function uses the current Clock's slot information to perform the necessary on-the-fly decryption or final calculation. This is a security measure that ensures the value can only be revealed at the correct time, making it impossible to know beforehand.
        This function call doesn't just return a number. It returns a Result, which is Rust's standard way of handling operations that might fail. It will either be:

        Ok(some_random_value): Success! Here is the [u8; 32] array representing the random number.
        Err(switchboard_error): Failure. Something went wrong. For example, the oracle might not have fulfilled the request yet, or the clock value might not be valid for revealing the number.
        In Plain English: This part says, "Using the current time as a key, unlock and reveal the final random number from the Switchboard data safe."

        Part 2: .map_err(|_| ErrorCode::RandomnessNotResolved)
        This part is all about making potential errors cleaner and more specific to our program.

        .map_err(): This is a function that operates on a Result. It does nothing if the result is Ok. If the result is Err, it allows you to transform, or "map," the error into a different one.
        |_|: This is a Rust closure, or an anonymous function. The _ is a placeholder that means, "I received an error from the previous step, but I don't care about its specific contents. I'm going to ignore it."
        ErrorCode::RandomnessNotResolved: This is one of our program's custom errors that we defined at the bottom of the file.
        So, this piece says: "If the get_value function failed, I don't want to bubble up the potentially complex Switchboard-specific error. Instead, I want to replace it with my own simple, clear error: RandomnessNotResolved."

        In Plain English: This translates a potentially technical failure message like "Decryption Failure Code X-47" into a simple, useful message for our program: "The randomness wasn't ready."

        Part 3: ?
        This is the question mark operator, also known as the "try" operator. It is a powerful piece of syntactic sugar in Rust for handling Result types.

        The ? at the end of the line does the following:

        If the Result is Ok(value): It unwraps the Result, takes the value out, and assigns it to the variable on the left (in this case, revealed_random_value). The program then continues to the next line.
        If the Result is Err(error): It immediately stops the execution of the choose_a_winner function and returns the Err(error) to the caller of the function.
        Putting It All Together
        So, the entire line reads like this logical statement:

        Attempt to get the final random value using the current clock.
        If this fails for any reason, convert the generic error into our specific RandomnessNotResolved error.
        Finally, check the result:
        If it was a success, put the revealed value into the revealed_random_value variable.
        If it was a failure (which is now our custom error), stop everything and exit the function, reporting that the randomness could not be resolved.*/
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

    // program needs to be mentioned because it is not actually owned by our token_lottery 
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

/////////////////

// Fourth Instruciton : 

Background Context
Remember, this code is inside the commit_a_winner function. Its job is not to pick the winner, but to publicly promise which random number source it will use before that number is revealed. The random number itself comes from an account created by the Switchboard oracle.

Line 1: let randomness_data = RandomnessAccountData::parse(ctx.accounts.randomness_account_data.data.borrow()).unwrap();
This line is all about taking raw, meaningless data from the blockchain and turning it into something our program can understand. Let's dissect it from the inside out:

ctx.accounts.randomness_account_data: This is the Switchboard account that was passed into our instruction. At this point, to our program, it's just a blob of raw, unstructured byte data.
.data: This specifically accesses that raw byte data.
.borrow(): In Rust, this is how you get a read-only reference to data without trying to take ownership of it. The Solana runtime "owns" all account data, so we can only borrow it to look at it.
RandomnessAccountData::parse(...): This is the magic from the Switchboard library. The parse function is specifically designed to take the raw byte data from a Switchboard randomness account and organize it into a proper, usable Rust struct called RandomnessAccountData. It knows exactly how to interpret the bytes to find things like the seed_slot, the value, etc.
.unwrap(): This is a simple form of error handling in Rust. The parse function might fail if the data isn't a valid Switchboard account. .unwrap() says, "If the parsing was successful, give me the structured data. If it failed, just stop the whole transaction."
In Plain English: This line says, "Take the raw data from the provided Switchboard account, and use Switchboard's own tools to translate it into an organized data structure that we can now work with."

Line 2: if randomness_data.seed_slot != clock.slot - 1 { ... }
This is a critical security and timing check. It ensures the random number is "fresh" and ready.

clock.slot: A "slot" is the fundamental unit of time on the Solana blockchain, roughly ~400 milliseconds. clock.slot gives us the current slot number (i.e., the current time).
randomness_data.seed_slot: This is a field inside the RandomnessAccountData we just parsed. It records the exact slot number when the Switchboard oracle generated the seed for this random number.
!= clock.slot - 1: The comparison checks if the seed_slot is not equal to the immediately preceding slot.
Why is this check so important?

This is the "Goldilocks" check. The random number seed must be in a state that is not too old and not too new. The perfect time is exactly one slot ago.

If the seed is too old (e.g., seed_slot was 10 slots ago), it might have been revealed already or could be considered stale.
If the seed is too new (e.g., seed_slot is the same as the current slot), it means the value isn't ready to be used yet.
This line enforces that we are committing to a random value that was seeded by the oracle in the last block and is now perfectly primed for commitment in the current block. The error code RandomnessAlreadyRevealed implies that if the timing isn't perfect, we assume something is wrong and the value might be known.

In Plain English: This line acts as a gatekeeper, saying "I will only commit to a random number that was prepared by the oracle in the immediately preceding moment. Otherwise, I can't trust its state."

Line 3: token_lottery.randomness_account = ctx.accounts.randomness_account_data.key();
This is the actual commitment. It's the most important action in this function.

ctx.accounts.randomness_account_data.key(): This gets the public key (the address) of the Switchboard account we've been validating.
token_lottery.randomness_account = ...: This takes that public key and saves it into a field within our program's main TokenLottery state account.
Why is this the commitment?

By writing the Switchboard account's public key into our own program's state, we are creating a permanent, unchangeable, on-chain record. It's a public promise.

Later, when we call the choose_a_winner function, the very first thing it will do is check that the randomness account provided in that call has the exact same public key that we stored right here. This prevents any "bait-and-switch" attempt where an admin might try to commit to one random number but reveal another.

In Plain English: This line says, "I have validated this specific source for our random number. Now, I am engraving its address into the lottery's permanent record to promise, in front of the entire world, that this is the one I will use."







@DOUBT : Now, i want to follow the randomness_account_data account , in the Accounts struct it is not initialized so far, so it does not exist . In the instruction, we pass it as a param to RandomnessAccountData::parse , here we borrow what is inside it. In the end we say, token_lottery.randomness_account = ctx.accounts.randomness_account_data.key(); . when is this account even being created , we are just borowwing the data in it and not even assigning it to anything , so how will it have the key to randomness account that we will use to find the winner




Show thinking
That is an absolutely fantastic question, and it gets to the heart of how Solana programs interact with each other and with the outside world. You are 100% correct in your observation: your Rust program itself does not create the randomness_account_data account.

The missing piece of the puzzle happens outside your program, on the "client-side" (e.g., in a TypeScript or JavaScript application) before your commit_a_winner function is ever called.

Here is the complete sequence of events:

The Two-Transaction Process
The lottery administrator (the authority) can't just call commit_a_winner out of the blue. They must first interact with the Switchboard oracle to get the process started.

Transaction 1: The Client Script Talks to Switchboard
The administrator runs a script that does the following:

Build a Request: The script builds a transaction to call the Switchboard Program. It says, "I need a fresh, on-demand random number."
Send the Request: The script sends this transaction to the Solana network.
Switchboard Acts: The Switchboard program receives this request and, within that transaction, it creates the RandomnessAccountData account. It finds an empty piece of memory on the blockchain, allocates space for the account, and fills it with initial data (like the seed_slot, but the final random value is still hidden/unrevealed).
The Result: At the end of this first transaction, two things are true:
The RandomnessAccountData account now exists on the blockchain at a specific public key (address).
The client-side script now knows this new public key.
This is the creation step you were looking for! It's done by the Switchboard program, at the request of your client script.

Transaction 2: The Client Script Talks to YOUR Program
Now that the randomness account exists, the administrator's script can finally call your program.

Build the commit_a_winner Instruction: The script now constructs the second transaction. This transaction will call your commit_a_winner function.

Provide the Accounts: When building the transaction, the script must provide all the accounts that your CommitWinner struct requires.

For payer, it provides the admin's wallet.
For token_lottery, it provides the address of your main lottery state.
And for randomness_account_data, it provides the public key of the account that was just created in Transaction 1.
Send the commit_a_winner Transaction: The script sends this complete transaction to the Solana network.

How Your Rust Code Interprets This
Now, when your Rust code for commit_a_winner executes, the Solana runtime has already loaded all the accounts that the client script provided.

pub randomness_account_data: UncheckedAccount<'info>
This is why it's an UncheckedAccount. You are telling Anchor, "Don't try to create this account (init) or validate its contents for me. Just trust that the client has passed in an account at this position. I will check it myself inside my function."

let randomness_data = RandomnessAccountData::parse(...)
This is your manual check. You are parsing the data of the account the client gave you to prove that it is, in fact, a valid Switchboard randomness account in the correct state.

... .key()
Since the account already exists on the blockchain (from Transaction 1), it has a public key. The .key() method simply reads the address of the account that was loaded into the transaction.

Analogy: Ordering a Custom Cake
Think of it like this:

Your commit_a_winner program is a Bakery. Its recipe knows how to "commit" to using a specific, special decoration.
The Switchboard program is a specialty Decoration Shop.
The randomness_account_data is a unique, edible gold leaf decoration.
The lottery administrator (the client) can't just go to the Bakery and say "make a cake with gold leaf." The Bakery doesn't make gold leaf.

Instead, the client must:

Go to the Decoration Shop (Switchboard) and buy the gold leaf. The shop creates it and gives it to the client in a box. (This is Transaction 1, creating the randomness account).
Take the box of gold leaf to the Bakery (your program) and say, "Please make me a cake and promise you'll use this specific gold leaf." (This is Transaction 2, calling commit_a_winner and passing in the account).
Your program then opens the box (parse), inspects the gold leaf to make sure it's real (if seed_slot ...), and then puts a label on the cake order that says "Use gold leaf from box #123" (token_lottery.randomness_account = ... .key()).





