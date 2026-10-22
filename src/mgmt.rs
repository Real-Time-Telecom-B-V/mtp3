//! Signalling network management and signalling link test messages.
//!
//! [`ManagementMessage`] is the content of an MSU with service indicator 0
//! (ITU-T Q.704 clause 15, plus the extended changeover signals of Q.2210 9.8.1
//! that a link with 24-bit sequence numbers uses), and [`TestMessage`] that of an
//! MSU with service indicator 1 (Q.707 5.4). Both encode to and decode from
//! [`Mtp3Msu::data`](crate::Mtp3Msu): the octets after the routing label,
//! starting with the heading code.
//!
//! The label of these messages is the routing label with the **signalling link
//! code** where a user part has the signalling link selection (Q.704 15.2), so
//! the SLC of a link-related message is [`Mtp3Msu::sls`](crate::Mtp3Msu), and
//! `0` for a message that is not about one link.
//!
//! Only the **ITU** formats (14-bit point codes) are implemented. The ANSI and
//! China formats differ in more than the point code width, and are refused with
//! [`ManagementError::UnsupportedVariant`] instead of being guessed at.
//!
//! ```
//! use mtp3::mgmt::ManagementMessage;
//! use mtp3::{Mtp3Msu, NetworkIndicator, PointCode, ServiceIndicator, Variant};
//!
//! // Transfer-prohibited for destination 2000, from 1000 to its neighbour 3000.
//! let pc = |v| PointCode::from_value(v, Variant::Itu).unwrap();
//! let tfp = ManagementMessage::Tfp { destination: pc(2000) };
//! let msu = Mtp3Msu {
//!     si: ServiceIndicator::SNM,
//!     ni: NetworkIndicator::National,
//!     mp: 0,
//!     opc: pc(1000),
//!     dpc: pc(3000),
//!     sls: 0, // the signalling link code: not about one link
//!     data: tfp.encode(Variant::Itu).unwrap(),
//! };
//! assert_eq!(msu.data, [0x14, 0xD0, 0x07]);
//! assert_eq!(ManagementMessage::decode(&msu.data, Variant::Itu).unwrap(), tfp);
//! ```

use thiserror::Error;

use crate::point_code::{PointCode, Variant};

/// Why a management or test message could not be encoded or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ManagementError {
    /// The octets end before the heading code or before a field it calls for.
    #[error("the message ends before its last field")]
    Truncated,
    /// A heading code Table 1/Q.704 (or Q.707 5.4) leaves spare.
    #[error("heading code H0 {h0} / H1 {h1} is not a known message")]
    UnknownHeading {
        /// The message group, the low half of the heading octet.
        h0: u8,
        /// The signal within the group, the high half of the heading octet.
        h1: u8,
    },
    /// The formats of this variant are not implemented (only ITU is), or a
    /// point code of another variant was handed in.
    #[error("management messages of the {0:?} variant are not implemented")]
    UnsupportedVariant(Variant),
    /// A signalling link test pattern outside 1 to 15 octets (Q.707 5.4).
    #[error("a test pattern is 1 to 15 octets, not {0}")]
    PatternLength(usize),
}

/// A signalling network management message (service indicator 0).
///
/// The variants are named by the signal abbreviations of Table 1/Q.704.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagementMessage {
    /// Changeover order: the 7-bit FSN of the last accepted MSU (15.4).
    Coo { fsn: u8 },
    /// Changeover acknowledgement (15.4).
    Coa { fsn: u8 },
    /// Extended changeover order: a 24-bit FSN (Q.2210 9.8.1).
    Xco { fsn: u32 },
    /// Extended changeover acknowledgement (Q.2210 9.8.1).
    Xca { fsn: u32 },
    /// Changeback declaration with its changeback code (15.5).
    Cbd { code: u8 },
    /// Changeback acknowledgement (15.5).
    Cba { code: u8 },
    /// Emergency changeover order (15.6).
    Eco,
    /// Emergency changeover acknowledgement (15.6).
    Eca,
    /// Signalling-route-set-congestion-test (15.16, national option).
    Rct,
    /// Transfer-controlled; `status` is the 2-bit congestion status a national
    /// network with several congestion states puts in the spare bits (15.15).
    Tfc { destination: PointCode, status: u8 },
    /// Transfer-prohibited (15.7).
    Tfp { destination: PointCode },
    /// Transfer-restricted (15.9, national option).
    Tfr { destination: PointCode },
    /// Transfer-allowed (15.8).
    Tfa { destination: PointCode },
    /// Signalling-route-set-test for a prohibited destination (15.10).
    Rst { destination: PointCode },
    /// Signalling-route-set-test for a restricted destination (15.10).
    Rsr { destination: PointCode },
    /// Link inhibit (15.11).
    Lin,
    /// Link uninhibit (15.11).
    Lun,
    /// Link inhibited acknowledgement (15.11).
    Lia,
    /// Link uninhibited acknowledgement (15.11).
    Lua,
    /// Link inhibit denied (15.11).
    Lid,
    /// Link force uninhibit (15.11).
    Lfu,
    /// Link local inhibit test (15.11).
    Llt,
    /// Link remote inhibit test (15.11).
    Lrt,
    /// Traffic restart allowed (15.12).
    Tra,
    /// Signalling-data-link-connection-order with the 12-bit identity of the
    /// data link (15.13).
    Dlc { link: u16 },
    /// Connection-successful (15.14).
    Css,
    /// Connection-not-successful (15.14).
    Cns,
    /// Connection-not-possible (15.14).
    Cnp,
    /// User part unavailable: the user part identity and the unavailability
    /// cause, 4 bits each (15.17).
    Upu {
        destination: PointCode,
        user_part: u8,
        cause: u8,
    },
}

/// Only the ITU formats are implemented.
fn itu_only(variant: Variant) -> Result<(), ManagementError> {
    match variant {
        Variant::Itu => Ok(()),
        other => Err(ManagementError::UnsupportedVariant(other)),
    }
}

/// A 14-bit destination, least significant octet first, with two bits above it.
fn put_destination(
    out: &mut Vec<u8>,
    destination: PointCode,
    upper: u8,
) -> Result<(), ManagementError> {
    itu_only(destination.variant())?;
    let value = destination.value() & 0x3FFF;
    out.push(value as u8);
    out.push((value >> 8) as u8 | (upper & 0x03) << 6);
    Ok(())
}

/// Read a 14-bit destination and the two bits above it.
fn destination(body: &[u8]) -> Result<(PointCode, u8), ManagementError> {
    let (low, high) = match body {
        [low, high, ..] => (*low, *high),
        _ => return Err(ManagementError::Truncated),
    };
    let value = u32::from(low) | u32::from(high & 0x3F) << 8;
    // Fourteen bits always fit the ITU point code.
    let pc = PointCode::from_value(value, Variant::Itu)
        .map_err(|_| ManagementError::UnsupportedVariant(Variant::Itu))?;
    Ok((pc, high >> 6))
}

impl ManagementMessage {
    /// The heading codes `(H0, H1)` of Table 1/Q.704.
    pub fn heading(&self) -> (u8, u8) {
        match self {
            Self::Coo { .. } => (1, 1),
            Self::Coa { .. } => (1, 2),
            Self::Xco { .. } => (1, 3),
            Self::Xca { .. } => (1, 4),
            Self::Cbd { .. } => (1, 5),
            Self::Cba { .. } => (1, 6),
            Self::Eco => (2, 1),
            Self::Eca => (2, 2),
            Self::Rct => (3, 1),
            Self::Tfc { .. } => (3, 2),
            Self::Tfp { .. } => (4, 1),
            Self::Tfr { .. } => (4, 3),
            Self::Tfa { .. } => (4, 5),
            Self::Rst { .. } => (5, 1),
            Self::Rsr { .. } => (5, 2),
            Self::Lin => (6, 1),
            Self::Lun => (6, 2),
            Self::Lia => (6, 3),
            Self::Lua => (6, 4),
            Self::Lid => (6, 5),
            Self::Lfu => (6, 6),
            Self::Llt => (6, 7),
            Self::Lrt => (6, 8),
            Self::Tra => (7, 1),
            Self::Dlc { .. } => (8, 1),
            Self::Css => (8, 2),
            Self::Cns => (8, 3),
            Self::Cnp => (8, 4),
            Self::Upu { .. } => (10, 1),
        }
    }

    /// The signal abbreviation of Table 1/Q.704, for logs and counters.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Coo { .. } => "COO",
            Self::Coa { .. } => "COA",
            Self::Xco { .. } => "XCO",
            Self::Xca { .. } => "XCA",
            Self::Cbd { .. } => "CBD",
            Self::Cba { .. } => "CBA",
            Self::Eco => "ECO",
            Self::Eca => "ECA",
            Self::Rct => "RCT",
            Self::Tfc { .. } => "TFC",
            Self::Tfp { .. } => "TFP",
            Self::Tfr { .. } => "TFR",
            Self::Tfa { .. } => "TFA",
            Self::Rst { .. } => "RST",
            Self::Rsr { .. } => "RSR",
            Self::Lin => "LIN",
            Self::Lun => "LUN",
            Self::Lia => "LIA",
            Self::Lua => "LUA",
            Self::Lid => "LID",
            Self::Lfu => "LFU",
            Self::Llt => "LLT",
            Self::Lrt => "LRT",
            Self::Tra => "TRA",
            Self::Dlc { .. } => "DLC",
            Self::Css => "CSS",
            Self::Cns => "CNS",
            Self::Cnp => "CNP",
            Self::Upu { .. } => "UPU",
        }
    }

    /// Encode to the octets after the routing label. Bits outside a field
    /// (the filler bit of a changeover order, spare bits) are sent as 0.
    pub fn encode(&self, variant: Variant) -> Result<Vec<u8>, ManagementError> {
        itu_only(variant)?;
        let (h0, h1) = self.heading();
        let mut out = Vec::with_capacity(4);
        out.push(h0 | h1 << 4);
        match *self {
            Self::Coo { fsn } | Self::Coa { fsn } => out.push(fsn & 0x7F),
            Self::Xco { fsn } | Self::Xca { fsn } => out.extend_from_slice(&fsn.to_le_bytes()[..3]),
            Self::Cbd { code } | Self::Cba { code } => out.push(code),
            Self::Tfc {
                destination,
                status,
            } => put_destination(&mut out, destination, status)?,
            Self::Tfp { destination }
            | Self::Tfr { destination }
            | Self::Tfa { destination }
            | Self::Rst { destination }
            | Self::Rsr { destination } => put_destination(&mut out, destination, 0)?,
            Self::Dlc { link } => {
                out.push(link as u8);
                out.push((link >> 8) as u8 & 0x0F);
            }
            Self::Upu {
                destination,
                user_part,
                cause,
            } => {
                put_destination(&mut out, destination, 0)?;
                out.push((user_part & 0x0F) | (cause & 0x0F) << 4);
            }
            Self::Eco
            | Self::Eca
            | Self::Rct
            | Self::Lin
            | Self::Lun
            | Self::Lia
            | Self::Lua
            | Self::Lid
            | Self::Lfu
            | Self::Llt
            | Self::Lrt
            | Self::Tra
            | Self::Css
            | Self::Cns
            | Self::Cnp => {}
        }
        Ok(out)
    }

    /// Decode the octets after the routing label. Octets after the last field
    /// of the message are ignored, as are bits outside a field.
    pub fn decode(data: &[u8], variant: Variant) -> Result<Self, ManagementError> {
        itu_only(variant)?;
        let (&heading, body) = data.split_first().ok_or(ManagementError::Truncated)?;
        let (h0, h1) = (heading & 0x0F, heading >> 4);
        let octet = || body.first().copied().ok_or(ManagementError::Truncated);
        let extended = || match body {
            [a, b, c, ..] => Ok(u32::from_le_bytes([*a, *b, *c, 0])),
            _ => Err(ManagementError::Truncated),
        };
        Ok(match (h0, h1) {
            (1, 1) => Self::Coo {
                fsn: octet()? & 0x7F,
            },
            (1, 2) => Self::Coa {
                fsn: octet()? & 0x7F,
            },
            (1, 3) => Self::Xco { fsn: extended()? },
            (1, 4) => Self::Xca { fsn: extended()? },
            (1, 5) => Self::Cbd { code: octet()? },
            (1, 6) => Self::Cba { code: octet()? },
            (2, 1) => Self::Eco,
            (2, 2) => Self::Eca,
            (3, 1) => Self::Rct,
            (3, 2) => {
                let (destination, status) = destination(body)?;
                Self::Tfc {
                    destination,
                    status,
                }
            }
            (4, 1) => Self::Tfp {
                destination: destination(body)?.0,
            },
            (4, 3) => Self::Tfr {
                destination: destination(body)?.0,
            },
            (4, 5) => Self::Tfa {
                destination: destination(body)?.0,
            },
            (5, 1) => Self::Rst {
                destination: destination(body)?.0,
            },
            (5, 2) => Self::Rsr {
                destination: destination(body)?.0,
            },
            (6, 1) => Self::Lin,
            (6, 2) => Self::Lun,
            (6, 3) => Self::Lia,
            (6, 4) => Self::Lua,
            (6, 5) => Self::Lid,
            (6, 6) => Self::Lfu,
            (6, 7) => Self::Llt,
            (6, 8) => Self::Lrt,
            (7, 1) => Self::Tra,
            (8, 1) => match body {
                [low, high, ..] => Self::Dlc {
                    link: u16::from(*low) | u16::from(high & 0x0F) << 8,
                },
                _ => return Err(ManagementError::Truncated),
            },
            (8, 2) => Self::Css,
            (8, 3) => Self::Cns,
            (8, 4) => Self::Cnp,
            (10, 1) => {
                let (destination, _) = destination(body)?;
                let last = body.get(2).copied().ok_or(ManagementError::Truncated)?;
                Self::Upu {
                    destination,
                    user_part: last & 0x0F,
                    cause: last >> 4,
                }
            }
            _ => return Err(ManagementError::UnknownHeading { h0, h1 }),
        })
    }
}

/// A signalling network testing and maintenance message (service indicator 1):
/// the signalling link test of ITU-T Q.707 2.2, formats of 5.4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestMessage {
    /// Signalling link test message, with the pattern its sender chose.
    Sltm { pattern: Vec<u8> },
    /// Signalling link test acknowledgement, with the pattern it answers.
    Slta { pattern: Vec<u8> },
}

impl TestMessage {
    /// The test pattern.
    pub fn pattern(&self) -> &[u8] {
        match self {
            Self::Sltm { pattern } | Self::Slta { pattern } => pattern,
        }
    }

    /// `SLTM` or `SLTA`, for logs and counters.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sltm { .. } => "SLTM",
            Self::Slta { .. } => "SLTA",
        }
    }

    /// Encode to the octets after the routing label: heading, the length of the
    /// pattern above four spare bits, the pattern.
    pub fn encode(&self, variant: Variant) -> Result<Vec<u8>, ManagementError> {
        itu_only(variant)?;
        let (h1, pattern) = match self {
            Self::Sltm { pattern } => (1u8, pattern),
            Self::Slta { pattern } => (2u8, pattern),
        };
        if !(1..=15).contains(&pattern.len()) {
            return Err(ManagementError::PatternLength(pattern.len()));
        }
        let mut out = Vec::with_capacity(2 + pattern.len());
        out.push(0x01 | h1 << 4);
        out.push((pattern.len() as u8) << 4);
        out.extend_from_slice(pattern);
        Ok(out)
    }

    /// Decode the octets after the routing label. The spare bits and anything
    /// after the pattern are ignored.
    pub fn decode(data: &[u8], variant: Variant) -> Result<Self, ManagementError> {
        itu_only(variant)?;
        let (heading, length) = match data {
            [heading, length, ..] => (*heading, usize::from(length >> 4)),
            _ => return Err(ManagementError::Truncated),
        };
        let (h0, h1) = (heading & 0x0F, heading >> 4);
        if h0 != 1 || !(1..=2).contains(&h1) {
            return Err(ManagementError::UnknownHeading { h0, h1 });
        }
        if length == 0 {
            return Err(ManagementError::PatternLength(0));
        }
        let pattern = data
            .get(2..2 + length)
            .ok_or(ManagementError::Truncated)?
            .to_vec();
        Ok(if h1 == 1 {
            Self::Sltm { pattern }
        } else {
            Self::Slta { pattern }
        })
    }
}
