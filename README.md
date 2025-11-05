# Blockchain Commons CBOR NaN-wrapped Byte String for Rust

<!--Guidelines: https://github.com/BlockchainCommons/secure-template/wiki -->

### _by Wolf McNally_

---

## Introduction

A reference implementation of [draft-mcnally-cbor-nan-bstr](https://datatracker.ietf.org/doc/draft-mcnally-cbor-nan-bstr/), which defines CBOR tag 102 for lossless transport of IEEE-754 NaN bit patterns. This crate enables exact round-tripping of all NaN attributes—sign bit, signaling/quiet bit, payload bits, and representation width—independent of the canonicalization policies that an ecosystem applies to floating-point numbers.

IEEE-754 purposefully leaves NaNs incomparable and allows implementations to use sign, signaling/quiet, and payload bits for implementation-defined purposes. When CBOR encoders perform preferred serialization or when deterministic profiles constrain encodings for predictability, the precise NaN formation can be lost. This tag treats a NaN as an opaque bit pattern, providing an explicit, interoperable mechanism to preserve all attributes across encode/decode.

The tag addresses concrete use cases including:

- **NaN boxing**: Schemes that use NaN payload space to embed tagged values and pointers
- **Deterministic profiles**: Ecosystems like [dCBOR](https://crates.io/crates/dcbor) that canonicalize to a single NaN but still need an escape hatch for exact preservation
- **Platform-specific signaling**: Applications that rely on specific NaN formations for error codes or diagnostics
- **Forensics and debugging**: Preserving exact bit patterns for analysis across transports

**NOTE:** Although this reference implementation is in the context of dCBOR, the nan-bstr specification including Tag 102 is *not* specific to dCBOR and can be implemented in any CBOR ecosystem.

## Getting Started

```toml
[dependencies]
cbor-nan-bstr = "0.1.0"
```

Basic usage examples:

```rust
use dcbor::prelude::*;
use cbor_nan_bstr::{NanBstr, NanWidth};

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
```

## Features

### NaN Width Support

The crate supports all four IEEE-754 binary interchange formats:

| Width     | Bytes | Type   | Constructor Method          |
| --------- | ----- | ------ | --------------------------- |
| binary16  | 2     | half   | `from_binary16_bits(u16)`   |
| binary32  | 4     | single | `from_binary32_bits(u32)`   |
| binary64  | 8     | double | `from_binary64_bits(u64)`   |
| binary128 | 16    | quad   | `from_binary128_bits(u128)` |

### NaN Attribute Inspection

The `NanBstr` type provides methods to inspect all NaN attributes without mutating bits:

- `width()`: Returns the IEEE-754 width (`NanWidth` enum)
- `sign()`: Returns the sign bit (true if negative)
- `is_quiet()`: Returns true if quiet NaN (not signaling)
- `is_signaling()`: Returns true if signaling NaN
- `fraction_bits()`: Returns the full significand/fraction field
- `payload_bits()`: Returns the NaN payload excluding the quiet/signaling bit
- `as_bytes()`: Returns the raw bytes in big-endian order

### CBOR Integration

Implements `CBORTagged`, `CBORTaggedEncodable`, and `CBORTaggedDecodable` traits from the `dcbor` crate for seamless integration with dCBOR encoding and decoding. The tag number is 102 as specified in the IETF draft.

### Interoperability with dCBOR

The dCBOR deterministic encoding profile allows only a single canonical NaN formation (half-width value with CBOR representation `0xf97e00`) and rejects others. This crate provides the explicit mechanism for exact NaN preservation when needed, allowing deterministic rules for numbers to remain intact while providing precise transport for exceptional cases.

## Specification

This crate implements [draft-mcnally-cbor-nan-bstr](https://datatracker.ietf.org/doc/draft-mcnally-cbor-nan-bstr/), which is currently an IETF Internet Draft in the CBOR Working Group.

The latest editor's copy is available at: [https://blockchaincommons.github.io/draft-mcnally-cbor-nan-bstr/draft-mcnally-cbor-nan-bstr.html](https://blockchaincommons.github.io/draft-mcnally-cbor-nan-bstr/draft-mcnally-cbor-nan-bstr.html)

## CBOR Diagnostic Notation Examples

The following examples show the diagnostic notation and hex encoding for each supported width:

| Width     | Description                                                          | Diagnostic Notation                        | Hex Encoding                             |
| --------- | -------------------------------------------------------------------- | ------------------------------------------ | ---------------------------------------- |
| binary16  | Half-precision quiet NaN (0x7E00)                                    | `102(h'7E00')`                             | `D866427E00`                             |
| binary32  | Single-precision quiet NaN with payload 0x000001                     | `102(h'7FC00001')`                         | `D866447FC00001`                         |
| binary64  | Double-precision signaling NaN with payload and sign bit set         | `102(h'FFF0000000000001')`                 | `D86648FFF0000000000001`                 |
| binary128 | Quad-precision quiet NaN with payload 0x0000000000000000000000000001 | `102(h'7FFF8000000000000000000000000001')` | `D866507FFF8000000000000000000000000001` |

In all cases, the content preserves sign, signaling/quiet, payload bits, and width exactly.

## Validation

A decoder that understands tag 102 enforces the following:

1. The enclosed byte string length is exactly 2, 4, 8, or 16 bytes
2. The bytes are interpreted in network byte order (big-endian)
3. The bit pattern is a valid NaN for the indicated width (exponent all ones, fraction field non-zero)
4. No normalization or canonicalization is performed by the tag processing itself

If any check fails, the decoder returns an error.

## Related Projects

- [dcbor](https://crates.io/crates/dcbor): Deterministic CBOR implementation for Rust
- [bc-tags](https://crates.io/crates/bc-tags): CBOR tags registry for Blockchain Commons projects
- [draft-mcnally-deterministic-cbor](https://datatracker.ietf.org/doc/draft-mcnally-deterministic-cbor/): dCBOR specification

## Status - Reference Implementation

`cbor-nan-bstr` is a reference implementation of the IETF Internet Draft. The specification is under active development in the CBOR Working Group. The API may change as the specification evolves.

This crate is intended for implementers who need to experiment with the nan-bstr tag or provide feedback on the specification. It should not be used in production until the IETF draft reaches a stable state.

See [Blockchain Commons' Development Phases](https://github.com/BlockchainCommons/Community/blob/master/release-path.md).

## Version History

### 0.1.0 - November 4, 2025

- Initial release.
- Reference implementation of draft-mcnally-cbor-nan-bstr.

## Financial Support

`cbor-nan-bstr` is a project of [Blockchain Commons](https://www.blockchaincommons.com/). We are proudly a "not-for-profit" social benefit corporation committed to open source & open development. Our work is funded entirely by donations and collaborative partnerships with people like you. Every contribution will be spent on building open tools, technologies, and techniques that sustain and advance blockchain and internet security infrastructure and promote an open web.

To financially support further development of `cbor-nan-bstr` and other projects, please consider becoming a Patron of Blockchain Commons through ongoing monthly patronage as a [GitHub Sponsor](https://github.com/sponsors/BlockchainCommons). You can also support Blockchain Commons with bitcoins at our [BTCPay Server](https://btcpay.blockchaincommons.com/).

## Contributing

We encourage public contributions through issues and pull requests! Please review [CONTRIBUTING.md](./CONTRIBUTING.md) for details on our development process. All contributions to this repository require a GPG signed [Contributor License Agreement](./CLA.md).

### Discussions

The best place to talk about Blockchain Commons and its projects is in our GitHub Discussions areas.

[**Gordian Developer Community**](https://github.com/BlockchainCommons/Gordian-Developer-Community/discussions). For standards and open-source developers who want to talk about interoperable wallet specifications, please use the Discussions area of the [Gordian Developer Community repo](https://github.com/BlockchainCommons/Gordian-Developer-Community/discussions). This is where you talk about Gordian specifications such as [Gordian Envelope](https://github.com/BlockchainCommons/Gordian/tree/master/Envelope#articles), [bc-shamir](https://github.com/BlockchainCommons/bc-shamir), [Sharded Secret Key Reconstruction](https://github.com/BlockchainCommons/bc-sskr), and [bc-ur](https://github.com/BlockchainCommons/bc-ur) as well as the larger [Gordian Architecture](https://github.com/BlockchainCommons/Gordian/blob/master/Docs/Overview-Architecture.md), its [Principles](https://github.com/BlockchainCommons/Gordian#gordian-principles) of independence, privacy, resilience, and openness, and its macro-architectural ideas such as functional partition (including airgapping, the original name of this community).

[**Gordian User Community**](https://github.com/BlockchainCommons/Gordian/discussions). For users of the Gordian reference apps, including [Gordian Coordinator](https://github.com/BlockchainCommons/iOS-GordianCoordinator), [Gordian Seed Tool](https://github.com/BlockchainCommons/GordianSeedTool-iOS), [Gordian Server](https://github.com/BlockchainCommons/GordianServer-macOS), [Gordian Wallet](https://github.com/BlockchainCommons/GordianWallet-iOS), and [SpotBit](https://github.com/BlockchainCommons/spotbit) as well as our whole series of [CLI apps](https://github.com/BlockchainCommons/Gordian/blob/master/Docs/Overview-Apps.md#cli-apps). This is a place to talk about bug reports and feature requests as well as to explore how our reference apps embody the [Gordian Principles](https://github.com/BlockchainCommons/Gordian#gordian-principles).

[**Blockchain Commons Discussions**](https://github.com/BlockchainCommons/Community/discussions). For developers, interns, and patrons of Blockchain Commons, please use the discussions area of the [Community repo](https://github.com/BlockchainCommons/Community) to talk about general Blockchain Commons issues, the intern program, or topics other than those covered by the [Gordian Developer Community](https://github.com/BlockchainCommons/Gordian-Developer-Community/discussions) or the [Gordian User Community](https://github.com/BlockchainCommons/Gordian/discussions).

### Other Questions & Problems

As an open-source, open-development community, Blockchain Commons does not have the resources to provide direct support of our projects. Please consider the discussions area as a locale where you might get answers to questions. Alternatively, please use this repository's [issues](./issues) feature. Unfortunately, we can not make any promises on response time.

If your company requires support to use our projects, please feel free to contact us directly about options. We may be able to offer you a contract for support from one of our contributors, or we might be able to point you to another entity who can offer the contractual support that you need.

### Credits

The following people directly contributed to this repository. You can add your name here by getting involved. The first step is learning how to contribute from our [CONTRIBUTING.md](./CONTRIBUTING.md) documentation.

| Name              | Role                     | Github                                           | Email                                 | GPG Fingerprint                                    |
| ----------------- | ------------------------ | ------------------------------------------------ | ------------------------------------- | -------------------------------------------------- |
| Christopher Allen | Principal Architect      | [@ChristopherA](https://github.com/ChristopherA) | \<ChristopherA@LifeWithAlacrity.com\> | FDFE 14A5 4ECB 30FC 5D22  74EF F8D3 6C91 3574 05ED |
| Wolf McNally      | Lead Researcher/Engineer | [@WolfMcNally](https://github.com/wolfmcnally)   | \<Wolf@WolfMcNally.com\>              | 9436 52EE 3844 1760 C3DC  3536 4B6C 2FCF 8947 80AE |

## Responsible Disclosure

We want to keep all of our software safe for everyone. If you have discovered a security vulnerability, we appreciate your help in disclosing it to us in a responsible manner. We are unfortunately not able to offer bug bounties at this time.

We do ask that you offer us good faith and use best efforts not to leak information or harm any user, their data, or our developer community. Please give us a reasonable amount of time to fix the issue before you publish it. Do not defraud our users or us in the process of discovery. We promise not to bring legal action against researchers who point out a problem provided they do their best to follow the these guidelines.

### Reporting a Vulnerability

Please report suspected security vulnerabilities in private via email to ChristopherA@BlockchainCommons.com (do not use this email for support). Please do NOT create publicly viewable issues for suspected security vulnerabilities.

The following keys may be used to communicate sensitive information to developers:

| Name              | Fingerprint                                        |
| ----------------- | -------------------------------------------------- |
| Christopher Allen | FDFE 14A5 4ECB 30FC 5D22  74EF F8D3 6C91 3574 05ED |
| Wolf McNally      | 9436 52EE 3844 1760 C3DC  3536 4B6C 2FCF 8947 80AE |

You can import a key by running the following command with that individual's fingerprint: `gpg --recv-keys "<fingerprint>"` Ensure that you put quotes around fingerprints that contain spaces.
