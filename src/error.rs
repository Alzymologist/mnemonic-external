#[derive(Debug)]
pub enum ErrorWordList {
    DamagedWord,
    InvalidChecksum,
    InvalidEntropy,
    InvalidWordNumber,
    NoWord,
    WordsNumber,
}
