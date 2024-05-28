#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(feature = "std")]
use std::fmt::{Result, Formatter, Display, Debug};

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum ErrorWordList {
    DamagedWord,
    InvalidChecksum,
    InvalidEntropy,
    InvalidWordNumber,
    NoWord,
    WordsNumber,
}

// TODO: provide actual error descriptions.
#[cfg(feature = "std")]
impl Display for ErrorWordList {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        <Self as Debug>::fmt(self, f)
    }
}
