//! SS7 point codes and the ITU/ANSI variants.
//!
//! A point code is the network address of a signalling point. Its bit width
//! and structured form depend on the SS7 variant:
//!
//! * **ITU**  — 14-bit, structured `zone-region-sp` as `3-8-3`.
//! * **ANSI** — 24-bit, structured `network-cluster-member` as `8-8-8`.
//! * **China**— 24-bit, same `8-8-8` layout as ANSI.
//!
//! Point codes parse from either the structured dotted/dashed form
//! (`"2-1-3"`) or a plain decimal (`"515"`), and render back to the
//! structured form for their variant.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// SS7 variant — fixes the point-code width and structured layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Variant {
    Itu,
    Ansi,
    China,
}

impl Variant {
    /// Component bit widths, most-significant first.
    pub const fn widths(self) -> [u8; 3] {
        match self {
            Variant::Itu => [3, 8, 3],                   // zone-region-sp
            Variant::Ansi | Variant::China => [8, 8, 8], // network-cluster-member
        }
    }

    /// Total point-code width in bits (14 for ITU, 24 for ANSI/China).
    pub const fn bits(self) -> u8 {
        let [a, b, c] = self.widths();
        a + b + c
    }

    /// Number of octets a point code occupies on the wire (rounded up).
    pub const fn octets(self) -> usize {
        (self.bits() as usize).div_ceil(8)
    }

    /// Largest value representable in this variant.
    pub const fn max_value(self) -> u32 {
        (1u32 << self.bits()) - 1
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PointCodeError {
    #[error("point code '{0}' has {1} components, expected 1 (decimal) or 3 (a-b-c)")]
    Components(String, usize),
    #[error("point code component '{0}' is not a number")]
    NotANumber(String),
    #[error("point code component {value} exceeds {bits}-bit field for {variant:?}")]
    ComponentTooLarge {
        value: u32,
        bits: u8,
        variant: Variant,
    },
    #[error("point code {value} exceeds the {bits}-bit range for {variant:?}")]
    OutOfRange {
        value: u32,
        bits: u8,
        variant: Variant,
    },
}

/// A point code plus the variant that gives it meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointCode {
    value: u32,
    variant: Variant,
}

impl PointCode {
    /// Build from a raw integer, validating it fits the variant's width.
    pub fn from_value(value: u32, variant: Variant) -> Result<Self, PointCodeError> {
        if value > variant.max_value() {
            return Err(PointCodeError::OutOfRange {
                value,
                bits: variant.bits(),
                variant,
            });
        }
        Ok(Self { value, variant })
    }

    /// Build from three structured components (most-significant first).
    pub fn from_components(parts: [u32; 3], variant: Variant) -> Result<Self, PointCodeError> {
        let widths = variant.widths();
        let mut value = 0u32;
        for (part, width) in parts.iter().zip(widths.iter()) {
            if *part >= (1u32 << width) {
                return Err(PointCodeError::ComponentTooLarge {
                    value: *part,
                    bits: *width,
                    variant,
                });
            }
            value = (value << width) | part;
        }
        Ok(Self { value, variant })
    }

    /// Parse the dotted/dashed structured form (`"2-1-3"`) or a plain
    /// decimal (`"515"`). Accepts `-` or `.` separators.
    pub fn parse(s: &str, variant: Variant) -> Result<Self, PointCodeError> {
        let parts: Vec<&str> = s.split(['-', '.']).collect();
        match parts.as_slice() {
            [single] => {
                let value: u32 = single
                    .trim()
                    .parse()
                    .map_err(|_| PointCodeError::NotANumber((*single).to_string()))?;
                Self::from_value(value, variant)
            }
            [a, b, c] => {
                let pa = a
                    .trim()
                    .parse()
                    .map_err(|_| PointCodeError::NotANumber((*a).to_string()))?;
                let pb = b
                    .trim()
                    .parse()
                    .map_err(|_| PointCodeError::NotANumber((*b).to_string()))?;
                let pc = c
                    .trim()
                    .parse()
                    .map_err(|_| PointCodeError::NotANumber((*c).to_string()))?;
                Self::from_components([pa, pb, pc], variant)
            }
            _ => Err(PointCodeError::Components(s.to_string(), parts.len())),
        }
    }

    /// The raw integer value (as it sits in the routing label, right-aligned).
    pub fn value(self) -> u32 {
        self.value
    }

    pub fn variant(self) -> Variant {
        self.variant
    }

    /// Decompose into the three structured components.
    pub fn components(self) -> [u32; 3] {
        let [wa, wb, wc] = self.variant.widths();
        let _ = wa;
        let a = self.value >> (wb + wc);
        let b = (self.value >> wc) & ((1u32 << wb) - 1);
        let c = self.value & ((1u32 << wc) - 1);
        [a, b, c]
    }
}

impl fmt::Display for PointCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c] = self.components();
        write!(f, "{a}-{b}-{c}")
    }
}

/// Parse from the default-variant-less string is intentionally NOT provided
/// — a point code is meaningless without its variant. Config types carry the
/// variant alongside and call [`PointCode::parse`].
impl PointCode {
    /// Convenience: parse with the ITU variant (the RTT default network).
    pub fn parse_itu(s: &str) -> Result<Self, PointCodeError> {
        Self::parse(s, Variant::Itu)
    }
}

/// Serde: a point code serialises as its `a-b-c` string. On the way in it is
/// stored variant-less as a raw string until a [`Variant`] resolves it; for
/// standalone (de)serialisation we assume ITU (the common case) and callers
/// re-resolve when the owning instance's variant is known.
impl Serialize for PointCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PointCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        PointCode::parse_itu(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for PointCode {
    type Err = PointCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PointCode::parse_itu(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itu_widths_and_range() {
        assert_eq!(Variant::Itu.bits(), 14);
        assert_eq!(Variant::Itu.octets(), 2);
        assert_eq!(Variant::Itu.max_value(), 0x3FFF);
        assert_eq!(Variant::Ansi.bits(), 24);
        assert_eq!(Variant::Ansi.octets(), 3);
        assert_eq!(Variant::Ansi.max_value(), 0xFF_FFFF);
    }

    #[test]
    fn itu_components_roundtrip() {
        // 2-1-3 ITU = (2<<11)|(1<<3)|3 = 4096 + 8 + 3 = 4107
        let pc = PointCode::from_components([2, 1, 3], Variant::Itu).unwrap();
        assert_eq!(pc.value(), 4107);
        assert_eq!(pc.components(), [2, 1, 3]);
        assert_eq!(pc.to_string(), "2-1-3");
    }

    #[test]
    fn itu_decimal_parse_matches_structured() {
        let a = PointCode::parse("5687", Variant::Itu).unwrap();
        assert_eq!(a.value(), 5687);
        let b = PointCode::parse(&a.to_string(), Variant::Itu).unwrap();
        assert_eq!(a, b); // decimal → structured → decimal is stable
    }

    #[test]
    fn ansi_888_layout() {
        // 1-1-5 ANSI = (1<<16)|(1<<8)|5 = 65536 + 256 + 5 = 65797
        let pc = PointCode::parse("1-1-5", Variant::Ansi).unwrap();
        assert_eq!(pc.value(), 65797);
        assert_eq!(pc.components(), [1, 1, 5]);
    }

    #[test]
    fn rejects_out_of_range() {
        // 0x3FFF is the ITU max; one more overflows.
        assert!(PointCode::from_value(0x4000, Variant::Itu).is_err());
        // ITU sp component is 3 bits → 8 is too large.
        assert_eq!(
            PointCode::from_components([0, 0, 8], Variant::Itu),
            Err(PointCodeError::ComponentTooLarge {
                value: 8,
                bits: 3,
                variant: Variant::Itu
            })
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(PointCode::parse("nope", Variant::Itu).is_err());
        assert!(PointCode::parse("1-2", Variant::Itu).is_err());
    }
}
