use core::fmt;
use dcbor::prelude::*;
use crate::{Error, NanWidth, Result};

/// A CBOR-friendly wrapper for an IEEE‑754 NaN bit pattern transported as a
/// byte string and tagged with CBOR tag 102 ("nan-bstr").
///
/// The enclosed bytes are kept exactly as given (big‑endian), and validity is
/// enforced at construction and when decoding from CBOR.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NanBstr(ByteString);

impl NanBstr {
    // ───────────────────────────── Constructors ─────────────────────────────

    /// Construct from a big‑endian byte slice (length 2, 4, 8, or 16).
    /// Validates that the bit pattern encodes a NaN of the corresponding width.
    pub fn from_be_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let b = bytes.as_ref();
        let width = NanWidth::from_len(b.len())?;
        if !is_nan_bits(width, b) {
            return Err(Error::NotANan);
        }
        Ok(Self(ByteString::from(b)))
    }

    /// Construct from a native-endian 16-bit bit pattern.
    pub fn from_binary16_bits(bits: u16) -> Result<Self> {
        Self::from_be_bytes(bits.to_be_bytes())
    }

    /// Construct from a native-endian 32-bit bit pattern.
    pub fn from_binary32_bits(bits: u32) -> Result<Self> {
        Self::from_be_bytes(bits.to_be_bytes())
    }

    /// Construct from a native-endian 64-bit bit pattern.
    pub fn from_binary64_bits(bits: u64) -> Result<Self> {
        Self::from_be_bytes(bits.to_be_bytes())
    }

    /// Construct from a native-endian 128-bit bit pattern (binary128 / f128).
    pub fn from_binary128_bits(bits: u128) -> Result<Self> {
        Self::from_be_bytes(bits.to_be_bytes())
    }

    /// Construct from two 64-bit words (high, low) representing binary128.
    pub fn from_binary128_words(high: u64, low: u64) -> Result<Self> {
        let bits = ((high as u128) << 64) | (low as u128);
        Self::from_binary128_bits(bits)
    }

    // ───────────────────────────── Accessors ────────────────────────────────

    /// Returns the width (binary16/32/64/128) encoded by the enclosed bytes.
    pub fn width(&self) -> NanWidth {
        NanWidth::from_len(self.0.len()).unwrap()
    }

    /// Returns the raw bytes in big‑endian order.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.data()
    }

    /// Returns the sign bit (true if set).
    pub fn sign(&self) -> bool {
        match self.width() {
            NanWidth::Binary16 => {
                (u16::from_be_bytes(self.0.data().try_into().unwrap()) >> 15)
                    & 1
                    == 1
            }
            NanWidth::Binary32 => {
                (u32::from_be_bytes(self.0.data().try_into().unwrap()) >> 31)
                    & 1
                    == 1
            }
            NanWidth::Binary64 => {
                (u64::from_be_bytes(self.0.data().try_into().unwrap()) >> 63)
                    & 1
                    == 1
            }
            NanWidth::Binary128 => {
                (u128::from_be_bytes(self.0.data().try_into().unwrap()) >> 127)
                    & 1
                    == 1
            }
        }
    }

    /// Returns true if the quiet/signaling indicator bit is 1 (quiet NaN).
    pub fn is_quiet(&self) -> bool {
        match self.width() {
            NanWidth::Binary16 => {
                (u16::from_be_bytes(self.0.data().try_into().unwrap()) >> 9) & 1
                    == 1
            }
            NanWidth::Binary32 => {
                (u32::from_be_bytes(self.0.data().try_into().unwrap()) >> 22)
                    & 1
                    == 1
            }
            NanWidth::Binary64 => {
                (u64::from_be_bytes(self.0.data().try_into().unwrap()) >> 51)
                    & 1
                    == 1
            }
            NanWidth::Binary128 => {
                (u128::from_be_bytes(self.0.data().try_into().unwrap()) >> 111)
                    & 1
                    == 1
            }
        }
    }

    /// Returns true if the NaN is signaling (quiet bit == 0).
    pub fn is_signaling(&self) -> bool {
        !self.is_quiet()
    }

    /// Returns the full significand/fraction field as bits (includes the
    /// quiet/signaling indicator bit in the MSB of the fraction field).
    pub fn fraction_bits(&self) -> u128 {
        match self.width() {
            NanWidth::Binary16 => {
                (u16::from_be_bytes(self.0.data().try_into().unwrap()) & 0x03FF)
                    as u128
            } // 10 bits
            NanWidth::Binary32 => {
                (u32::from_be_bytes(self.0.data().try_into().unwrap())
                    & 0x007F_FFFF) as u128
            } // 23 bits
            NanWidth::Binary64 => {
                (u64::from_be_bytes(self.0.data().try_into().unwrap())
                    & 0x000F_FFFF_FFFF_FFFF) as u128
            } // 52 bits
            NanWidth::Binary128 => {
                let bits =
                    u128::from_be_bytes(self.0.data().try_into().unwrap());
                bits & ((1u128 << 112) - 1)
            }
        }
    }

    /// Returns the NaN payload bits excluding the quiet/signaling indicator
    /// bit (i.e., the remaining fraction bits beneath the MSB of the
    /// significand). This is the portion commonly treated as user payload.
    pub fn payload_bits(&self) -> u128 {
        match self.width() {
            NanWidth::Binary16 => self.fraction_bits() & ((1u128 << 9) - 1), /* 9 bits */
            NanWidth::Binary32 => self.fraction_bits() & ((1u128 << 22) - 1), /* 22 bits */
            NanWidth::Binary64 => self.fraction_bits() & ((1u128 << 51) - 1), /* 51 bits */
            NanWidth::Binary128 => self.fraction_bits() & ((1u128 << 111) - 1), /* 111 bits */
        }
    }

    /// If the width is binary128, return the full 128-bit bit pattern.
    pub fn to_binary128_bits(&self) -> Option<u128> {
        match self.width() {
            NanWidth::Binary128 => {
                Some(u128::from_be_bytes(self.0.data().try_into().unwrap()))
            }
            _ => None,
        }
    }
}

// ───────────────────────── CBOR Tagged Implementation ───────────────────────

impl CBORTagged for NanBstr {
    fn cbor_tags() -> Vec<Tag> {
        tags_for_values(&[bc_tags::TAG_NAN_BSTR])
    }
}

impl CBORTaggedEncodable for NanBstr {
    fn untagged_cbor(&self) -> CBOR {
        CBOR::from(self.0.clone())
    }
}

impl CBORTaggedDecodable for NanBstr {
    fn from_untagged_cbor(cbor: CBOR) -> dcbor::Result<Self> {
        let bs: ByteString =
            cbor.try_into().map_err(|_| dcbor::Error::WrongType)?;
        Ok(NanBstr::from_be_bytes(bs.data())?)
    }
}

impl From<NanBstr> for CBOR {
    fn from(value: NanBstr) -> Self {
        value.tagged_cbor()
    }
}

impl TryFrom<CBOR> for NanBstr {
    type Error = dcbor::Error;
    fn try_from(cbor: CBOR) -> dcbor::Result<Self> {
        Self::from_tagged_cbor(cbor)
    }
}

// ──────────────────────── f32/f64 Conversions ───────────────────────────────

impl TryFrom<f32> for NanBstr {
    type Error = Error;
    fn try_from(value: f32) -> Result<Self> {
        if !value.is_nan() {
            return Err(Error::NotANan);
        }
        Self::from_binary32_bits(value.to_bits())
    }
}

impl TryFrom<NanBstr> for f32 {
    type Error = Error;
    fn try_from(value: NanBstr) -> Result<Self> {
        if value.width() != NanWidth::Binary32 {
            return Err(Error::InvalidLength(value.0.len()));
        }
        let bits = u32::from_be_bytes(value.0.data().try_into().unwrap());
        Ok(f32::from_bits(bits))
    }
}

impl TryFrom<f64> for NanBstr {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        if !value.is_nan() {
            return Err(Error::NotANan);
        }
        Self::from_binary64_bits(value.to_bits())
    }
}

impl TryFrom<NanBstr> for f64 {
    type Error = Error;
    fn try_from(value: NanBstr) -> Result<Self> {
        if value.width() != NanWidth::Binary64 {
            return Err(Error::InvalidLength(value.0.len()));
        }
        let bits = u64::from_be_bytes(value.0.data().try_into().unwrap());
        Ok(f64::from_bits(bits))
    }
}

// ───────────────────────────────── Display ──────────────────────────────────

impl fmt::Display for NanBstr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (w, s, q, frac, payload) = (
            self.width(),
            self.sign(),
            self.is_quiet(),
            self.fraction_bits(),
            self.payload_bits(),
        );
        let wbits = w.len() * 8;
        write!(
            f,
            "NaN[{}]: {} {} frac=0x{:x} payload=0x{:x}",
            wbits,
            if s { "-" } else { "+" },
            if q { "quiet" } else { "signaling" },
            frac,
            payload,
        )
    }
}

// ────────────────────────────── Internals ───────────────────────────────────

fn is_nan_bits(width: NanWidth, be_bytes: &[u8]) -> bool {
    match width {
        NanWidth::Binary16 => {
            let b = <[u8; 2]>::try_from(be_bytes).unwrap();
            let bits = u16::from_be_bytes(b);
            let exp = (bits >> 10) & 0x1F; // 5 exponent bits all ones = 0x1F
            let frac = bits & 0x03FF; // 10 fraction bits
            exp == 0x1F && frac != 0
        }
        NanWidth::Binary32 => {
            let b = <[u8; 4]>::try_from(be_bytes).unwrap();
            let bits = u32::from_be_bytes(b);
            let exp = (bits >> 23) & 0xFF; // 8 exponent bits all ones = 0xFF
            let frac = bits & 0x007F_FFFF; // 23 fraction bits
            exp == 0xFF && frac != 0
        }
        NanWidth::Binary64 => {
            let b = <[u8; 8]>::try_from(be_bytes).unwrap();
            let bits = u64::from_be_bytes(b);
            let exp = (bits >> 52) & 0x7FF; // 11 exponent bits all ones = 0x7FF
            let frac = bits & 0x000F_FFFF_FFFF_FFFF; // 52 fraction bits
            exp == 0x7FF && frac != 0
        }
        NanWidth::Binary128 => {
            let b = <[u8; 16]>::try_from(be_bytes).unwrap();
            let bits = u128::from_be_bytes(b);
            let exp = (bits >> 112) & 0x7FFF; // 15 exponent bits all ones = 0x7FFF
            let frac = bits & ((1u128 << 112) - 1); // 112 fraction bits
            exp == 0x7FFF && frac != 0
        }
    }
}
