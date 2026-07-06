//! The MTP3-User Service Access Point (Q.701 MTP-TRANSFER + network management).
//!
//! [`Mtp3UserPart`] is the seam SCCP (and ISUP) are written against — the
//! MTP3-user interface. Two things implement it: the MTP3 network layer (over
//! M2PA links) and M3UA (RFC 4666). A user is generic over the trait, so it
//! runs unchanged over either.

use thiserror::Error;

use crate::point_code::{PointCode, Variant};

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

impl Mtp3Msu {
    /// The SIO octet: SI in bits 0..3, message priority in bits 4..5 (spare, so
    /// zero, for ITU international; the priority field for ANSI), NI in bits 6..7.
    fn sio(&self) -> u8 {
        (self.ni.bits() << 6) | ((self.mp & 0x03) << 4) | (self.si.0 & 0x0F)
    }

    /// Encode to the on-wire MSU: SIO octet, then the routing label, then the SIF
    /// (the MTP3-user payload). The routing-label layout is variant-specific.
    ///
    /// * **ITU** (Q.704) — one 32-bit little-endian word,
    ///   `DPC(14) | OPC(14) << 14 | SLS(4) << 28`.
    /// * **ANSI / China** (T1.111) — DPC (3 octets) then OPC (3 octets), each
    ///   point code least-significant octet first (member, cluster, network),
    ///   then a one-octet SLS. `variant` fixes the layout; the point codes are
    ///   masked to the variant width, so an over-wide value can't bleed into an
    ///   adjacent field.
    ///
    /// ```
    /// use mtp3::{Mtp3Msu, NetworkIndicator, PointCode, ServiceIndicator, Variant};
    ///
    /// let msu = Mtp3Msu {
    ///     si: ServiceIndicator::SCCP,
    ///     ni: NetworkIndicator::International,
    ///     mp: 0,
    ///     opc: PointCode::from_components([2, 1, 3], Variant::Itu).unwrap(),
    ///     dpc: PointCode::from_components([4, 2, 1], Variant::Itu).unwrap(),
    ///     sls: 7,
    ///     data: vec![0x09, 0x81, 0x03, 0x0e, 0x19],
    /// };
    /// let wire = msu.encode(Variant::Itu);
    /// assert_eq!(wire, [0x03, 0x11, 0xe0, 0x02, 0x74, 0x09, 0x81, 0x03, 0x0e, 0x19]);
    /// assert_eq!(Mtp3Msu::decode(&wire, Variant::Itu).unwrap(), msu);
    /// ```
    pub fn encode(&self, variant: Variant) -> Vec<u8> {
        let sio = self.sio();
        match variant {
            Variant::Itu => {
                let dpc = self.dpc.value() & 0x3FFF;
                let opc = self.opc.value() & 0x3FFF;
                let sls = (self.sls as u32) & 0x0F;
                let label = dpc | (opc << 14) | (sls << 28);
                let mut out = Vec::with_capacity(5 + self.data.len());
                out.push(sio);
                out.extend_from_slice(&label.to_le_bytes());
                out.extend_from_slice(&self.data);
                out
            }
            Variant::Ansi | Variant::China => {
                let dpc = (self.dpc.value() & 0x00FF_FFFF).to_le_bytes();
                let opc = (self.opc.value() & 0x00FF_FFFF).to_le_bytes();
                let mut out = Vec::with_capacity(8 + self.data.len());
                out.push(sio);
                out.extend_from_slice(&dpc[..3]);
                out.extend_from_slice(&opc[..3]);
                out.push(self.sls);
                out.extend_from_slice(&self.data);
                out
            }
        }
    }

    /// Decode an on-wire MSU (as produced by [`encode`](Mtp3Msu::encode)) under
    /// the given `variant`. Recovers the SIO fields, the OPC/DPC/SLS routing
    /// label, and the SIF as the payload.
    ///
    /// Errors with [`Mtp3Error::Decode`] if the input is shorter than the SIO
    /// plus the variant's routing label (5 octets for ITU, 8 for ANSI/China).
    pub fn decode(bytes: &[u8], variant: Variant) -> Result<Self, Mtp3Error> {
        let label_len = match variant {
            Variant::Itu => 4,
            Variant::Ansi | Variant::China => 7,
        };
        let header = 1 + label_len;
        if bytes.len() < header {
            return Err(Mtp3Error::Decode(format!(
                "need at least {header} octets for the SIO + {variant:?} routing label, got {}",
                bytes.len()
            )));
        }

        let sio = bytes[0];
        let si = ServiceIndicator(sio & 0x0F);
        let mp = (sio >> 4) & 0x03;
        let ni = NetworkIndicator::from_bits(sio >> 6);

        let (opc, dpc, sls) = match variant {
            Variant::Itu => {
                let label = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
                let dpc = label & 0x3FFF;
                let opc = (label >> 14) & 0x3FFF;
                let sls = ((label >> 28) & 0x0F) as u8;
                (opc, dpc, sls)
            }
            Variant::Ansi | Variant::China => {
                let dpc = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], 0]);
                let opc = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], 0]);
                (opc, dpc, bytes[7])
            }
        };

        let to_pc = |value: u32| {
            PointCode::from_value(value, variant).map_err(|e| Mtp3Error::Decode(e.to_string()))
        };
        Ok(Self {
            si,
            ni,
            mp,
            opc: to_pc(opc)?,
            dpc: to_pc(dpc)?,
            sls,
            data: bytes[header..].to_vec(),
        })
    }
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
    /// The bytes handed to [`Mtp3Msu::decode`] are not a well-formed MSU
    /// (too short for the SIO + routing label, or an out-of-range point code).
    #[error("MSU decode: {0}")]
    Decode(String),
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

    // ── On-wire codec (Q.704 ITU / T1.111 ANSI) ─────────────────────────────

    /// The proven ITU routing label the SS7 stack hand-rolled before this crate
    /// owned the format: OPC 2-1-3 (4107), DPC 4-2-1 (8209), SLS 7, SI SCCP,
    /// NI international, over an SCCP SIF. SIO = 0x03; the 32-bit little-endian
    /// label is `8209 | (4107 << 14) | (7 << 28) = 0x7402_E011` → `11 E0 02 74`.
    fn itu_kav() -> (Mtp3Msu, [u8; 10]) {
        let msu = Mtp3Msu {
            si: ServiceIndicator::SCCP,
            ni: NetworkIndicator::International,
            mp: 0,
            opc: PointCode::from_components([2, 1, 3], Variant::Itu).unwrap(),
            dpc: PointCode::from_components([4, 2, 1], Variant::Itu).unwrap(),
            sls: 7,
            data: vec![0x09, 0x81, 0x03, 0x0e, 0x19],
        };
        let wire = [
            0x03, // SIO: NI=0 (intl), spare=0, SI=3 (SCCP)
            0x11, 0xe0, 0x02, 0x74, // routing label, little-endian
            0x09, 0x81, 0x03, 0x0e, 0x19, // SIF
        ];
        (msu, wire)
    }

    #[test]
    fn itu_encode_is_byte_exact() {
        let (msu, wire) = itu_kav();
        assert_eq!(msu.encode(Variant::Itu), wire);
    }

    /// Independently reproduce the hand-rolled formula (the one still living in
    /// ss7-stack and siphon-sigtran) and confirm the crate emits the same bytes,
    /// so this is byte-identity to the proven layout, not a copied constant.
    #[test]
    fn itu_encode_matches_hand_rolled_formula() {
        let (msu, _) = itu_kav();
        let (dpc, opc, sls) = (8209u32, 4107u32, 7u32);
        let label: u32 = dpc | (opc << 14) | (sls << 28);
        let mut expected = vec![0x03];
        expected.extend_from_slice(&label.to_le_bytes());
        expected.extend_from_slice(&msu.data);
        assert_eq!(msu.encode(Variant::Itu), expected);
    }

    #[test]
    fn itu_decode_recovers_fields() {
        let (msu, wire) = itu_kav();
        let back = Mtp3Msu::decode(&wire, Variant::Itu).unwrap();
        assert_eq!(back, msu);
        assert_eq!(back.opc.value(), 4107);
        assert_eq!(back.dpc.value(), 8209);
        assert_eq!(back.sls, 7);
        assert_eq!(back.si, ServiceIndicator::SCCP);
        assert_eq!(back.ni, NetworkIndicator::International);
    }

    #[test]
    fn itu_round_trip() {
        let (msu, _) = itu_kav();
        assert_eq!(
            Mtp3Msu::decode(&msu.encode(Variant::Itu), Variant::Itu).unwrap(),
            msu
        );
    }

    /// Hand-built T1.111 vector. OPC 1-2-3 = (1<<16)|(2<<8)|3 = 0x01_0203, laid
    /// out least-significant octet first as `03 02 01`; DPC 4-5-6 = 0x04_0506 as
    /// `06 05 04`. SI SCCP, NI national (2), priority 1 → SIO = (2<<6)|(1<<4)|3 =
    /// 0x93. SLS 0x1F in its own octet. Label order is DPC, then OPC, then SLS.
    fn ansi_kav() -> (Mtp3Msu, [u8; 10]) {
        let msu = Mtp3Msu {
            si: ServiceIndicator::SCCP,
            ni: NetworkIndicator::National,
            mp: 1,
            opc: PointCode::from_components([1, 2, 3], Variant::Ansi).unwrap(),
            dpc: PointCode::from_components([4, 5, 6], Variant::Ansi).unwrap(),
            sls: 0x1F,
            data: vec![0xaa, 0xbb],
        };
        let wire = [
            0x93, // SIO: NI=2 (national), MP=1, SI=3 (SCCP)
            0x06, 0x05, 0x04, // DPC 4-5-6, LSO first
            0x03, 0x02, 0x01, // OPC 1-2-3, LSO first
            0x1F, // SLS
            0xaa, 0xbb, // SIF
        ];
        (msu, wire)
    }

    #[test]
    fn ansi_encode_is_byte_exact() {
        let (msu, wire) = ansi_kav();
        assert_eq!(msu.encode(Variant::Ansi), wire);
    }

    #[test]
    fn ansi_decode_recovers_fields() {
        let (msu, wire) = ansi_kav();
        let back = Mtp3Msu::decode(&wire, Variant::Ansi).unwrap();
        assert_eq!(back, msu);
        assert_eq!(back.opc.components(), [1, 2, 3]);
        assert_eq!(back.dpc.components(), [4, 5, 6]);
        assert_eq!(back.mp, 1);
        assert_eq!(back.sls, 0x1F);
        assert_eq!(back.ni, NetworkIndicator::National);
    }

    #[test]
    fn ansi_round_trip() {
        let (msu, _) = ansi_kav();
        assert_eq!(
            Mtp3Msu::decode(&msu.encode(Variant::Ansi), Variant::Ansi).unwrap(),
            msu
        );
    }

    /// China takes the 24-bit ANSI-style layout, so the wire bytes match.
    #[test]
    fn china_matches_ansi_layout() {
        let msu = Mtp3Msu {
            si: ServiceIndicator::ISUP,
            ni: NetworkIndicator::National,
            mp: 0,
            opc: PointCode::from_components([7, 8, 9], Variant::China).unwrap(),
            dpc: PointCode::from_components([1, 2, 3], Variant::China).unwrap(),
            sls: 5,
            data: vec![0x01],
        };
        let ansi_shaped = Mtp3Msu {
            opc: PointCode::from_value(msu.opc.value(), Variant::Ansi).unwrap(),
            dpc: PointCode::from_value(msu.dpc.value(), Variant::Ansi).unwrap(),
            ..msu.clone()
        };
        assert_eq!(
            msu.encode(Variant::China),
            ansi_shaped.encode(Variant::Ansi)
        );
        assert_eq!(
            Mtp3Msu::decode(&msu.encode(Variant::China), Variant::China).unwrap(),
            msu
        );
    }

    #[test]
    fn sio_packs_si_ni_mp() {
        // SI/NI/priority land in the right SIO fields and survive the round trip.
        for (si, ni, mp, want) in [
            (
                ServiceIndicator::SCCP,
                NetworkIndicator::International,
                0u8,
                0x03u8,
            ),
            (ServiceIndicator::ISUP, NetworkIndicator::National, 0, 0x85),
            (ServiceIndicator::SCCP, NetworkIndicator::National, 3, 0xB3),
            (
                ServiceIndicator(0),
                NetworkIndicator::NationalSpare,
                2,
                0xE0,
            ),
        ] {
            let msu = Mtp3Msu {
                si,
                ni,
                mp,
                opc: PointCode::from_value(1, Variant::Itu).unwrap(),
                dpc: PointCode::from_value(2, Variant::Itu).unwrap(),
                sls: 0,
                data: vec![],
            };
            let wire = msu.encode(Variant::Itu);
            assert_eq!(wire[0], want, "SIO for si={si:?} ni={ni:?} mp={mp}");
            let back = Mtp3Msu::decode(&wire, Variant::Itu).unwrap();
            assert_eq!((back.si, back.ni, back.mp), (si, ni, mp));
        }
    }

    #[test]
    fn empty_sif_round_trips() {
        for variant in [Variant::Itu, Variant::Ansi, Variant::China] {
            let msu = Mtp3Msu {
                si: ServiceIndicator::SCCP,
                ni: NetworkIndicator::International,
                mp: 0,
                opc: PointCode::from_value(1, variant).unwrap(),
                dpc: PointCode::from_value(2, variant).unwrap(),
                sls: 0,
                data: vec![],
            };
            let wire = msu.encode(variant);
            // SIO + routing label, no SIF. ITU packs the label into 4 octets;
            // ANSI/China lay it out as DPC(3) + OPC(3) + SLS(1) = 7.
            let label_len = if variant == Variant::Itu { 4 } else { 7 };
            assert_eq!(wire.len(), 1 + label_len);
            let back = Mtp3Msu::decode(&wire, variant).unwrap();
            assert!(back.data.is_empty());
            assert_eq!(back, msu);
        }
    }

    #[test]
    fn decode_rejects_short_input() {
        // ITU needs 5 octets for SIO + label; ANSI/China need 8.
        assert!(matches!(
            Mtp3Msu::decode(&[0x03, 0x11, 0xe0, 0x02], Variant::Itu),
            Err(Mtp3Error::Decode(_))
        ));
        assert!(Mtp3Msu::decode(&[0x03, 0x11, 0xe0, 0x02, 0x74], Variant::Itu).is_ok());
        assert!(matches!(
            Mtp3Msu::decode(&[0x93, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01], Variant::Ansi),
            Err(Mtp3Error::Decode(_))
        ));
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
