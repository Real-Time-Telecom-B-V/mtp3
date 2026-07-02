//! The MTP3-User Service Access Point (Q.701 MTP-TRANSFER + network management).
//!
//! [`Mtp3UserPart`] is the seam SCCP (and ISUP) are written against — the
//! MTP3-user interface. Two things implement it: the MTP3 network layer (over
//! M2PA links) and M3UA (RFC 4666). A user is generic over the trait, so it
//! runs unchanged over either.

use thiserror::Error;

use crate::point_code::PointCode;

/// Service Indicator — the low nibble of the SIO, naming the MTP3-user that
/// owns the message. Opaque `u8`; the well-known values have constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServiceIndicator(pub u8);

impl ServiceIndicator {
    /// Signalling Network Management messages.
    pub const SNM: Self = Self(0);
    /// Signalling Network Testing and Maintenance.
    pub const SNT: Self = Self(1);
    /// SCCP.
    pub const SCCP: Self = Self(3);
    /// Telephone User Part.
    pub const TUP: Self = Self(4);
    /// ISDN User Part.
    pub const ISUP: Self = Self(5);

    /// The well-known name, if any.
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::SNM => "SNM",
            Self::SNT => "SNT",
            Self::SCCP => "SCCP",
            Self::TUP => "TUP",
            Self::ISUP => "ISUP",
            _ => return None,
        })
    }
}

/// Network Indicator — the top two bits of the SIO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NetworkIndicator {
    International = 0,
    InternationalSpare = 1,
    National = 2,
    NationalSpare = 3,
}

impl NetworkIndicator {
    pub fn from_bits(v: u8) -> Self {
        match v & 0b11 {
            0 => Self::International,
            1 => Self::InternationalSpare,
            2 => Self::National,
            _ => Self::NationalSpare,
        }
    }
    pub fn bits(self) -> u8 {
        self as u8
    }
}

/// An MTP3 Message Signal Unit as seen at the MTP3-user boundary — the routing
/// label (OPC/DPC/SLS), the SIO fields (SI/NI/priority), and the user payload.
/// The parameters of the MTP-TRANSFER primitive (Q.701).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mtp3Msu {
    /// Service Indicator (which MTP3-user; SCCP = 3).
    pub si: ServiceIndicator,
    /// Network Indicator (international / national).
    pub ni: NetworkIndicator,
    /// Message priority (0–3; ANSI/China use it, ITU ignores it).
    pub mp: u8,
    /// Originating Point Code.
    pub opc: PointCode,
    /// Destination Point Code.
    pub dpc: PointCode,
    /// Signalling Link Selection (load-sharing / sequencing key).
    pub sls: u8,
    /// The MTP3-user payload (e.g. an encoded SCCP message).
    pub data: Vec<u8>,
}

/// Network-management status for a destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mtp3Status {
    /// The destination is congested (transfer-controlled) at the given level.
    Congested { level: u8 },
    /// A user part at the destination is unavailable (MTP3 UPU / M3UA DUPU).
    UserPartUnavailable { si: ServiceIndicator, cause: u8 },
}

/// An inbound event from the network layer: a delivered MSU (MTP-TRANSFER
/// indication) or a network-management indication (PAUSE/RESUME/STATUS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mtp3Event {
    /// MTP-TRANSFER.indication — an MSU addressed to us.
    Transfer(Mtp3Msu),
    /// The destination became inaccessible (MTP-PAUSE / M3UA DUNA).
    Pause { affected: PointCode },
    /// The destination became accessible (MTP-RESUME / M3UA DAVA).
    Resume { affected: PointCode },
    /// A status change for the destination (MTP-STATUS / M3UA SCON / DUPU).
    Status {
        affected: PointCode,
        status: Mtp3Status,
    },
}

/// Errors surfaced by an [`Mtp3UserPart`].
#[derive(Debug, Error)]
pub enum Mtp3Error {
    /// The destination point code has no route / is unreachable.
    #[error("destination {0} is unreachable")]
    Unreachable(PointCode),
    /// The underlying transport failed.
    #[error("transport error: {0}")]
    Transport(String),
    /// The provider is shutting down / the link is out of service.
    #[error("mtp3-user part is not in service")]
    OutOfService,
}

/// The MTP3-User Service Access Point.
///
/// Implemented by the MTP3 network layer (native, over M2PA links) and by M3UA
/// (as an IP adaptation of the same service). SCCP and ISUP are written generic
/// over this trait, so they run unchanged over either transport. A
/// composite router that dispatches by destination can itself implement the
/// trait, unifying several providers behind one SAP.
#[async_trait::async_trait]
pub trait Mtp3UserPart: Send + Sync {
    /// MTP-TRANSFER.request — hand an MSU to the network for delivery.
    async fn send(&self, msu: Mtp3Msu) -> Result<(), Mtp3Error>;

    /// Await the next inbound event — a delivered MSU or a network-management
    /// indication (PAUSE/RESUME/STATUS).
    async fn recv(&self) -> Result<Mtp3Event, Mtp3Error>;

    /// Whether a destination point code is currently reachable.
    fn is_available(&self, dpc: PointCode) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_code::{PointCode, Variant};

    #[test]
    fn service_indicator_names() {
        assert_eq!(ServiceIndicator::SCCP.0, 3);
        assert_eq!(ServiceIndicator::ISUP.name(), Some("ISUP"));
        assert_eq!(ServiceIndicator(9).name(), None);
    }

    #[test]
    fn network_indicator_bits() {
        assert_eq!(
            NetworkIndicator::from_bits(0),
            NetworkIndicator::International
        );
        assert_eq!(NetworkIndicator::from_bits(2), NetworkIndicator::National);
        assert_eq!(NetworkIndicator::National.bits(), 2);
    }

    #[test]
    fn msu_construct() {
        let msu = Mtp3Msu {
            si: ServiceIndicator::SCCP,
            ni: NetworkIndicator::International,
            mp: 0,
            opc: PointCode::from_value(1, Variant::Itu).unwrap(),
            dpc: PointCode::from_value(2, Variant::Itu).unwrap(),
            sls: 5,
            data: vec![0x09, 0x80, 0x03],
        };
        assert_eq!(msu.si, ServiceIndicator::SCCP);
        assert_eq!(msu.dpc.value(), 2);
    }

    /// A trivial in-memory `Mtp3UserPart` proves the trait is object-safe and
    /// usable behind `dyn` (needed for the STP router).
    struct Loopback;
    #[async_trait::async_trait]
    impl Mtp3UserPart for Loopback {
        async fn send(&self, _msu: Mtp3Msu) -> Result<(), Mtp3Error> {
            Ok(())
        }
        async fn recv(&self) -> Result<Mtp3Event, Mtp3Error> {
            Err(Mtp3Error::OutOfService)
        }
        fn is_available(&self, _dpc: PointCode) -> bool {
            true
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let p: Box<dyn Mtp3UserPart> = Box::new(Loopback);
        assert!(p.is_available(PointCode::from_value(1, Variant::Itu).unwrap()));
    }
}
