use std::f64;

use cbor_nan_bstr::{NanBstr, NanWidth};
use dcbor::prelude::*;

#[test]
fn half_quiet_nan_roundtrip() {
    // binary16 quiet NaN 0x7E00 (q=1, payload=0)
    let n = NanBstr::from_binary16_bits(0x7E00).unwrap();
    assert_eq!(
        n.to_string(),
        "NaN[16]: + quiet frac=0x200 payload=0x0"
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
        "NaN[32]: - signaling frac=0x1 payload=0x1"
    );
}

#[test]
fn double_quiet_nan_roundtrip() {
    // binary64 quiet NaN: sign=0, exp=0x7FF, quiet bit=1, payload=0x123
    let bits: u64 = 0x7FF8_0000_0000_0123; // qNaN with payload
    let n = NanBstr::from_binary64_bits(bits).unwrap();
    assert_eq!(
        n.to_string(),
        "NaN[64]: + quiet frac=0x8000000000123 payload=0x123"
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
        "NaN[128]: + quiet frac=0x8000000000000000000000000000 payload=0x0"
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
        "NaN[128]: + signaling frac=0x1 payload=0x1"
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

#[test]
fn read_me() {
    // Create from a native f32 NaN
    let nan = f32::NAN;
    let nan_bstr = NanBstr::try_from(nan).unwrap();

    // Inspect NaN attributes
    assert_eq!(nan_bstr.width(), NanWidth::Binary32);
    assert!(nan_bstr.is_quiet());
    assert!(!nan_bstr.sign());

    // Encode to CBOR (tagged with 102)
    let cbor = CBOR::from(nan_bstr);
    assert_eq!(cbor.diagnostic(), "102(h'7fc00000')");

    // Decode from CBOR
    let decoded = NanBstr::try_from(cbor).unwrap();

    // Convert back to native types
    let f32_value = f32::try_from(decoded).unwrap();
    assert!(f32_value.is_nan());

    // Create from specific bit patterns
    let quiet_nan = NanBstr::from_binary32_bits(0x7FC00001).unwrap();
    assert_eq!(quiet_nan.to_string(), "NaN[32]: + quiet frac=0x400001 payload=0x1");
    assert_eq!(quiet_nan.to_cbor().diagnostic(), "102(h'7fc00001')");

    let signaling_nan = NanBstr::from_binary64_bits(0xFFF0000000000001).unwrap();
    assert_eq!(signaling_nan.to_string(), "NaN[64]: - signaling frac=0x1 payload=0x1");
    assert_eq!(signaling_nan.to_cbor().diagnostic(), "102(h'fff0000000000001')");

    // Non-NaNs cannot be converted
    assert!(NanBstr::try_from(1.0f32).is_err());
    assert!(NanBstr::try_from(f64::INFINITY).is_err());
}
