//! A CBOR Tag for Lossless Transport of IEEE‑754 NaN Bit Patterns
//! Implements draft‑mcnally‑cbor‑nan‑bstr semantics for dCBOR.
//!
//! This module defines a `NanBstr` tagged value that carries the exact
//! bit pattern of an IEEE‑754 NaN in a CBOR byte string, tagged with 102.
//!
//! The enclosed byte string MUST be exactly 2, 4, 8, or 16 bytes long and is
//! interpreted in network byte order (big‑endian). Its bits MUST encode
//! a NaN for the corresponding interchange width; no canonicalization of
//! sign, quiet/signaling, payload, or width is performed here.
//!
//! NOTE: This includes **binary128 (f128)** support without using any native
//! `f128` type: APIs accept/return raw bit patterns as `u128` or `[u8; 16]`.

mod nan_bstr;
pub use nan_bstr::*;
mod nan_width;
pub use nan_width::*;
mod error;
pub use error::*;
