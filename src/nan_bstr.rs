// A CBOR Tag for Lossless Transport of IEEE‑754 NaN Bit Patterns
// Implements draft‑mcnally‑cbor‑nan‑bstr semantics for dCBOR.
//
// This module defines a `NanBstr` tagged value that carries the exact
// bit pattern of an IEEE‑754 NaN in a CBOR byte string, tagged with 102.
//
// The enclosed byte string MUST be exactly 2, 4, 8, or 16 bytes long and is
// interpreted in network byte order (big‑endian). Its bits MUST encode
// a NaN for the corresponding interchange width; no canonicalization of
// sign, quiet/signaling, payload, or width is performed here.
//
// NOTE: This includes **binary128 (f128)** support without using any native
// `f128` type: APIs accept/return raw bit patterns as `u128` or `[u8; 16]`.
//
// NOTE: To complete integration with dcbor’s global tag registry and get
// name-based diagnostics, add registration lines for TAG_NAN_BSTR in
// `src/tags.rs` similar to TAG_DATE. This file focuses on the type and
// encode/decode logic; registration is kept orthogonal.

#![allow(clippy::len_without_is_empty)]

use core::fmt;

use dcbor::prelude::*;

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

impl NanWidth {
    #[inline]
    pub fn from_len(len: usize) -> Result<Self> {
        match len {
            2 => Ok(Self::Binary16),
            4 => Ok(Self::Binary32),
            8 => Ok(Self::Binary64),
            16 => Ok(Self::Binary128),
            _ => Err(Error::InvalidLength(len)),
        }
    }

    #[inline]
    pub fn len(self) -> usize {
        match self {
            Self::Binary16 => 2,
            Self::Binary32 => 4,
            Self::Binary64 => 8,
            Self::Binary128 => 16,
        }
    }
}

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
    #[inline]
    pub fn width(&self) -> NanWidth {
        NanWidth::from_len(self.0.len()).unwrap()
    }

    /// Returns the raw bytes in big‑endian order.
    #[inline]
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
    #[inline]
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
        let wbits = match w {
            NanWidth::Binary16 => 16,
            NanWidth::Binary32 => 32,
            NanWidth::Binary64 => 64,
            NanWidth::Binary128 => 128,
        };
        write!(
            f,
            "NaN[{}]: sign={}, {}, frac=0x{:x}, payload=0x{:x}",
            wbits,
            if s { 1 } else { 0 },
            if q { "quiet" } else { "signaling" },
            frac,
            payload,
        )
    }
}

// ────────────────────────────── Internals ───────────────────────────────────

#[inline]
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

// ─────────────────────────────── Tests ──────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_quiet_nan_roundtrip() {
        // binary16 quiet NaN 0x7E00 (q=1, payload=0)
        let n = NanBstr::from_binary16_bits(0x7E00).unwrap();
        assert_eq!(
            n.to_string(),
            "NaN[16]: sign=0, quiet, frac=0x200, payload=0x0"
        );

        let cbor = CBOR::from(n.clone());
        assert_eq!(cbor.diagnostic(), "102(h'7e00')");

        let back: NanBstr = cbor.try_into().unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn single_signaling_nan_validates() {
        // binary32 signaling NaN: exp=0xFF, q=0, frac!=0, sign=1
        let bits: u32 = 0xFF80_0001; // negative sNaN with minimal payload
        let n = NanBstr::from_binary32_bits(bits).unwrap();
        assert_eq!(
            n.to_string(),
            "NaN[32]: sign=1, signaling, frac=0x1, payload=0x1"
        );
    }

    #[test]
    fn double_quiet_nan_roundtrip() {
        // binary64 quiet NaN: sign=0, exp=0x7FF, quiet bit=1, payload=0x123
        let bits: u64 = 0x7FF8_0000_0000_0123; // qNaN with payload
        let n = NanBstr::from_binary64_bits(bits).unwrap();
        assert_eq!(
            n.to_string(),
            "NaN[64]: sign=0, quiet, frac=0x8000000000123, payload=0x123"
        );

        let cbor = CBOR::from(n.clone());
        assert_eq!(cbor.diagnostic(), "102(h'7ff8000000000123')");

        let back: NanBstr = cbor.try_into().unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn double_nan_rejects_infinity() {
        // +Infinity: exp=all ones, frac=0 — must be rejected
        let inf: u64 = 0x7FF0_0000_0000_0000;
        assert!(NanBstr::from_binary64_bits(inf).is_err());
    }

    #[test]
    fn quad_quiet_nan_roundtrip() {
        // binary128 quiet NaN: sign=0, exp=0x7FFF, quiet bit=1, payload=0
        let bits: u128 = (0x7FFFu128 << 112) | (1u128 << 111);
        let n = NanBstr::from_binary128_bits(bits).unwrap();
        assert_eq!(
            n.to_string(),
            "NaN[128]: sign=0, quiet, frac=0x8000000000000000000000000000, payload=0x0"
        );

        let cbor = CBOR::from(n.clone());
        let back: NanBstr = cbor.try_into().unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn quad_signaling_nan_validates() {
        // binary128 signaling NaN: exp=0x7FFF, quiet bit=0, payload=1
        let bits: u128 = (0x7FFFu128 << 112) | 1u128;
        let n = NanBstr::from_binary128_bits(bits).unwrap();
        assert_eq!(
            n.to_string(),
            "NaN[128]: sign=0, signaling, frac=0x1, payload=0x1"
        );
    }

    #[test]
    fn quad_nan_rejects_infinity() {
        // +Infinity for binary128: exp=all ones, frac=0 — must be rejected
        let bits: u128 = 0x7FFFu128 << 112; // exponent all ones, fraction 0
        assert!(NanBstr::from_binary128_bits(bits).is_err());
    }

    #[test]
    fn encoding_tag_value_is_102() {
        let n = NanBstr::from_binary32_bits(0x7FC0_0001).unwrap(); // qNaN
        let cbor = n.tagged_cbor();
        // Expect outer tag value == 102
        let (tag, _value) = cbor.try_into_tagged_value().unwrap();
        assert_eq!(tag.value(), bc_tags::TAG_NAN_BSTR);
    }

    #[test]
    fn f32_to_nanbstr_roundtrip() {
        let nan_f32 = f32::NAN;
        let n = NanBstr::try_from(nan_f32).unwrap();
        assert_eq!(n.width(), NanWidth::Binary32);

        let back = f32::try_from(n).unwrap();
        assert!(back.is_nan());
    }

    #[test]
    fn f32_try_from_rejects_non_nan() {
        assert!(NanBstr::try_from(1.0f32).is_err());
        assert!(NanBstr::try_from(f32::INFINITY).is_err());
        assert!(NanBstr::try_from(0.0f32).is_err());
    }

    #[test]
    fn f32_try_from_nanbstr_rejects_wrong_width() {
        let n = NanBstr::from_binary64_bits(0x7FF8_0000_0000_0000).unwrap();
        assert!(f32::try_from(n).is_err());
    }

    #[test]
    fn f64_to_nanbstr_roundtrip() {
        let nan_f64 = f64::NAN;
        let n = NanBstr::try_from(nan_f64).unwrap();
        assert_eq!(n.width(), NanWidth::Binary64);

        let back = f64::try_from(n).unwrap();
        assert!(back.is_nan());
    }

    #[test]
    fn f64_try_from_rejects_non_nan() {
        assert!(NanBstr::try_from(1.0f64).is_err());
        assert!(NanBstr::try_from(f64::INFINITY).is_err());
        assert!(NanBstr::try_from(0.0f64).is_err());
    }

    #[test]
    fn f64_try_from_nanbstr_rejects_wrong_width() {
        let n = NanBstr::from_binary32_bits(0x7FC0_0001).unwrap();
        assert!(f64::try_from(n).is_err());
    }
}
