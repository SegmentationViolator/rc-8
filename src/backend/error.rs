use std::error;
use std::fmt;

use super::instruction;

#[derive(Debug)]
pub struct BackendError {
    pub instruction: Option<(usize, Option<instruction::Instruction>)>,
    pub kind: BackendErrorKind,
}

#[derive(Debug)]
pub enum BackendErrorKind {
    MemoryOverflow,
    ProgramInvalid,
    ProgramNotLoaded,
    StackOverflow,
    StackUnderflow,
    UnrecognizedInstruction,
    UnrecognizedSprite,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.instruction {
            Some((index, Some(instruction))) => write!(
                f,
                "instruction {} at 0x{:03x}, {}",
                instruction, index, self.kind
            ),
            Some((index, None)) => write!(f, "at 0x{:x}, {}", index, self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl fmt::Display for BackendErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MemoryOverflow => "attempt to access invalid memory address",
                Self::ProgramInvalid => "attempt to load invalid program",
                Self::ProgramNotLoaded => "attempt to run without loading any program",
                Self::StackOverflow => "attempt to call a coroutine when the stack is full",
                Self::StackUnderflow => "attempt to return when the stack is empty",
                Self::UnrecognizedInstruction => "unrecognized instruction",
                Self::UnrecognizedSprite => "attempt to load unrecognized sprite",
            },
        )
    }
}

impl error::Error for BackendError {}
