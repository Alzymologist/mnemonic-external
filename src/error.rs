#[cfg(feature = "std")]
use std::string::String;

#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "std")]
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[cfg(not(feature = "std"))]
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub enum ErrorMnemonic {
    DamagedWord,
    InvalidChecksum,
    InvalidEntropy,
    InvalidWordNumber,
    NoWord,
    WordsNumber,
}

impl ErrorMnemonic {
    fn error_text(&self) -> String {
        match &self {
            ErrorMnemonic::DamagedWord => String::from("Unable to extract a word from the word list."),
            ErrorMnemonic::InvalidChecksum => String::from("Invalid text mnemonic: the checksum does not match."),
            ErrorMnemonic::InvalidEntropy => String::from("Unable to calculate the mnemonic from entropy. Invalid entropy length."),
            ErrorMnemonic::InvalidWordNumber => String::from("Ordinal number for word requested is higher than total number of words in the word list."),
            ErrorMnemonic::NoWord => String::from("Requested word in not in the word list."),
            ErrorMnemonic::WordsNumber => String::from("Invalid text mnemonic: unexpected number of words."),
        }
    }
}

impl Display for ErrorMnemonic {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.error_text())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ErrorMnemonic {}
