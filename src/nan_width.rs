use crate::{Error, Result};

/// Width of the underlying IEEE‑754 representation carried in the byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NanWidth {
    /// 2-octet IEEE‑754 binary16 (aka half, f16)
    Binary16,
    /// 4-octet IEEE‑754 binary32 (aka single, f32)
    Binary32,
    /// 8-octet IEEE‑754 binary64 (aka double, f64)
    Binary64,
    /// 16-octet IEEE‑754 binary128 (aka quad, f128)
    Binary128,
}

#[allow(clippy::len_without_is_empty)]
impl NanWidth {
    pub fn from_len(len: usize) -> Result<Self> {
        match len {
            2 => Ok(Self::Binary16),
            4 => Ok(Self::Binary32),
            8 => Ok(Self::Binary64),
            16 => Ok(Self::Binary128),
            _ => Err(Error::InvalidLength(len)),
        }
    }

    pub fn len(self) -> usize {
        match self {
            Self::Binary16 => 2,
            Self::Binary32 => 4,
            Self::Binary64 => 8,
            Self::Binary128 => 16,
        }
    }
}
