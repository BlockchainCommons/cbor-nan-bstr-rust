#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("CBOR error ({0})")]
    Cbor(#[from] dcbor::Error),

    #[error("invalid NaN length: expected 2, 4, 8, or 16 bytes, got {0} bytes")]
    InvalidLength(usize),

    #[error("not a NaN bit pattern")]
    NotANan,
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for String {
    fn from(err: Error) -> Self { err.to_string() }
}

impl From<Error> for dcbor::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Cbor(err) => err,
            _ => dcbor::Error::msg(err),
        }
    }
}
