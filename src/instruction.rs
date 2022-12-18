use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

pub enum StudInstruction {
    AddStudent { name: String, message: String },
    UpdateStudent { name: String, message: String },
    AddComment { message: String },
}

#[derive(BorshDeserialize)]
struct StudInstructionPayload {
    name: String,
    message: String,
}

#[derive(BorshDeserialize)]
struct CommentPayload {
    message: String,
}

impl StudInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match variant {
            0 => {
                let payload = StudInstructionPayload::try_from_slice(rest).unwrap();
                Self::AddStudent {
                    name: payload.name,
                    message: payload.message,
                }
            }
            1 => {
                let payload = StudInstructionPayload::try_from_slice(rest).unwrap();
                Self::UpdateStudent {
                    name: payload.name,
                    message: payload.message,
                }
            }
            2 => {
                let payload = CommentPayload::try_from_slice(rest).unwrap();
                Self::AddComment {
                    message: payload.message,
                }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
