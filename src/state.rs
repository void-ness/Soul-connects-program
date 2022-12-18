use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StudAccountState {
    pub discriminator: String,
    pub initialized: bool,
    pub reviewer: Pubkey,
    pub name: String,
    pub message: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StudCommentCounter {
    pub discriminator: String,
    pub initialized: bool,
    pub counter: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StudComment {
    pub discriminator: String,
    pub initialized: bool,
    pub introduction: Pubkey,
    pub commenter: Pubkey,
    pub comment: String,
    pub count: u64,
}

impl Sealed for StudAccountState {}

impl IsInitialized for StudAccountState {
    fn is_initialized(&self) -> bool {
        self.is_initialized()
    }
}

impl IsInitialized for StudCommentCounter {
    fn is_initialized(&self) -> bool {
        self.is_initialized()
    }
}

impl IsInitialized for StudComment {
    fn is_initialized(&self) -> bool {
        self.is_initialized()
    }
}

impl StudAccountState {
    pub const DISCRIMINATOR: &'static str = "introduction";

    pub fn get_account_size(name: String, message: String) -> usize {
        return (4 + StudAccountState::DISCRIMINATOR.len())
            + 1
            + 32
            + (4 + name.len())
            + (4 + message.len());
    }
}

impl StudCommentCounter {
    pub const DISCRIMINATOR: &'static str = "counter";

    pub const SIZE: usize = (4 + StudCommentCounter::DISCRIMINATOR.len()) + 1 + 8;
}

impl StudComment {
    pub const DISCRIMINATOR: &'static str = "comment";

    pub fn get_account_size(comment: String) -> usize {
        return (4 + StudComment::DISCRIMINATOR.len()) + 1 + 32 + 32 + (4 + comment.len()) + 8;
    }
}
