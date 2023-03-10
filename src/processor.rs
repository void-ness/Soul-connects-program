use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

use spl_associated_token_account::get_associated_token_address;
use spl_token::{instruction::initialize_mint, ID as TOKEN_PROGRAM_ID};

use borsh::BorshSerialize;
use std::convert::TryInto;

use crate::error::ReviewError;
use crate::instruction::StudInstruction;
use crate::state::{StudAccountState, StudComment, StudCommentCounter};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = StudInstruction::unpack(instruction_data)?;
    match instruction {
        StudInstruction::AddStudent { name, message } => {
            add_stud(program_id, accounts, name, message)
        }
        StudInstruction::UpdateStudent { name, message } => {
            update_stud(program_id, accounts, name, message)
        }
        StudInstruction::AddComment { message } => add_comment(program_id, accounts, message),
        StudInstruction::InitializeMint => initialize_token_mint(program_id, accounts),
    }
}

pub fn update_stud(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    message: String,
) -> ProgramResult {
    msg!("updating account data");
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    if pda_account.owner != program_id {
        msg!("Invalid Owner");
        return Err(ProgramError::IllegalOwner);
    }

    if !initializer.is_signer {
        msg!("missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(&[initializer.key.as_ref()], program_id);

    if pda != *pda_account.key {
        msg!("invalid pda");
        return Err(ReviewError::InvalidPDA.into());
    }

    msg!("pda created {}", pda);

    let total_len: usize = StudAccountState::get_account_size(name.clone(), message.clone());
    if total_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::DataLimit.into());
    }

    msg!("unpacking data from acccount");
    let mut account_data =
        try_from_slice_unchecked::<StudAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("borrowed account data");

    if !account_data.initialized {
        msg!("Account not initialized");
        return Err(ReviewError::UninitializedAcc.into());
    }

    account_data.name = name;
    account_data.message = message;

    msg!("serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("account serialized");
    Ok(())
}

pub fn add_stud(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    message: String,
) -> ProgramResult {
    // iterating over accounts
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;

    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;

    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    if !initializer.is_signer {
        msg!("missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // deriving the program address to be used for storing data
    let (pda, bump_seed) = Pubkey::find_program_address(&[initializer.key.as_ref()], program_id);

    if pda != *pda_account.key {
        msg!("pda provided is incorrect");
        return Err(ReviewError::InvalidPDA.into());
    }

    // calculating account space for rent
    let account_len: usize = 1000;
    if StudAccountState::get_account_size(name.clone(), message.clone()) > account_len {
        msg!("length of data exceeds the specified amount");
        return Err(ReviewError::DataLimit.into());
    }
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_account.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&[initializer.key.as_ref(), &[bump_seed]]],
    )?;

    msg!("pda created {}", pda);
    msg!("unpacking data from acccount");
    let mut account_data =
        try_from_slice_unchecked::<StudAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("borrowed account data");

    if account_data.initialized {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    account_data.discriminator = StudAccountState::DISCRIMINATOR.to_string();
    account_data.reviewer = *initializer.key;
    account_data.initialized = true;
    account_data.name = name;
    account_data.message = message;

    msg!("serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("account serialized");

    msg!("Create comment counter");
    let rent = Rent::get()?;
    let counter_rent_lamports = rent.minimum_balance(StudCommentCounter::SIZE);

    let (counter, counter_bump) =
        Pubkey::find_program_address(&[pda.as_ref(), "comment".as_ref()], program_id);

    if counter != *pda_counter.key {
        msg!("invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_counter.key,
            counter_rent_lamports,
            StudCommentCounter::SIZE.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_counter.clone(),
            system_program.clone(),
        ],
        &[&[pda.as_ref(), "comment".as_ref(), &[counter_bump]]],
    )?;

    msg!("Comment Counter Created");

    let mut counter_data =
        try_from_slice_unchecked::<StudCommentCounter>(&pda_counter.data.borrow()).unwrap();

    if counter_data.initialized {
        msg!("account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    msg!("Borrowed Account Data");

    counter_data.discriminator = StudCommentCounter::DISCRIMINATOR.to_string();
    counter_data.counter = 0;
    counter_data.initialized = true;

    msg!("comment count: {}", counter_data.counter);
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;
    msg!("counter Account Serialized");

    msg!("deriving mint authority");
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    if *token_mint.key != mint_pda {
        msg!("Incorrect token mint");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *mint_auth.key != mint_auth_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *user_ata.key != get_associated_token_address(initializer.key, token_mint.key) {
        msg!("Incorrect token mint");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::InvalidPDA.into());
    }

    msg!("Minting 10 tokens to User associated token account");
    invoke_signed(
        // Instruction
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            10 * LAMPORTS_PER_SOL,
        )?,
        // Account_infos
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        // Seeds
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;

    Ok(())
}

pub fn add_comment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    message: String,
) -> ProgramResult {
    msg!("Adding Comment....");
    msg!("Comment: {}", message);

    let account_info_iter = &mut accounts.iter();
    let commenter = next_account_info(account_info_iter)?;
    let pda_review = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let pda_comment = next_account_info(account_info_iter)?;

    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;

    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    let mut counter_data =
        try_from_slice_unchecked::<StudCommentCounter>(&pda_counter.data.borrow()).unwrap();

    let comment_account_len: usize = StudComment::get_account_size(message.clone());

    let rent = Rent::get()?;
    let comment_rent_lamports = rent.minimum_balance(comment_account_len);

    let (comment_account, comment_account_bump) = Pubkey::find_program_address(
        &[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if comment_account != *pda_comment.key {
        msg!("invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    invoke_signed(
        &system_instruction::create_account(
            commenter.key,
            pda_comment.key,
            comment_rent_lamports,
            comment_account_len.try_into().unwrap(),
            program_id,
        ),
        &[
            commenter.clone(),
            pda_comment.clone(),
            system_program.clone(),
        ],
        &[&[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
            &[comment_account_bump],
        ]],
    )?;
    msg!("comment account created");

    msg!("borrowing data");
    let mut comment_account_data =
        try_from_slice_unchecked::<StudComment>(&pda_comment.data.borrow()).unwrap();

    // if comment_account_data.initialized {
    //     msg!("Account already initialized");
    //     return Err(ProgramError::AccountAlreadyInitialized);
    // }
    comment_account_data.comment = message;
    comment_account_data.discriminator = StudComment::DISCRIMINATOR.to_string();
    comment_account_data.initialized = true;
    comment_account_data.introduction = *pda_review.key; //the introduction which is being commented upon
    comment_account_data.commenter = *commenter.key; //the commenter
    comment_account_data.count = counter_data.counter + 1;

    msg!("serializing account");
    comment_account_data.serialize(&mut &mut pda_comment.data.borrow_mut()[..])?;
    msg!("comment account serialized");

    msg!("serializing counter account");
    counter_data.counter += 1;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;
    msg!("counter account serialized");

    msg!("deriving mint authority");
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    if *token_mint.key != mint_pda {
        msg!("Incorrect token mint");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *mint_auth.key != mint_auth_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *user_ata.key != get_associated_token_address(commenter.key, token_mint.key) {
        msg!("Incorrect token mint");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::InvalidPDA.into());
    }

    msg!("Minting 5 tokens to User associated token account");
    invoke_signed(
        // Instruction
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            5 * LAMPORTS_PER_SOL,
        )?,
        // Account_infos
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        // Seeds
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;

    Ok(())
}

pub fn initialize_token_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let sysvar_rent = next_account_info(account_info_iter)?;

    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, _mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);

    msg!("Token mint: {:?}", mint_pda);
    msg!("Mint authority: {:?}", mint_auth_pda);

    if mint_pda != *token_mint.key {
        msg!("Incorrect token mint account");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *mint_auth.key != mint_auth_pda {
        msg!("Incorrect mint auth account");
        return Err(ReviewError::InvalidPDA.into());
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(82);

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            token_mint.key,
            rent_lamports,
            82,
            token_program.key,
        ),
        &[
            initializer.clone(),
            token_mint.clone(),
            system_program.clone(),
        ],
        &[&[b"token_mint", &[mint_bump]]],
    )?;

    msg!("Created token mint account");

    invoke_signed(
        &initialize_mint(
            token_program.key,
            token_mint.key,
            mint_auth.key,
            Option::None,
            9,
        )?,
        &[token_mint.clone(), sysvar_rent.clone(), mint_auth.clone()],
        &[&[b"token_mint", &[mint_bump]]],
    )?;

    msg!("Initialized token mint");

    Ok(())
}
