//! # ss7-mtp3
//!
//! The SS7 **MTP3-User Service Access Point** and point-code types — the shared
//! seam of an SS7 network stack.
//!
//! [`Mtp3UserPart`] is the interface SCCP (and ISUP) are written against. It is
//! implemented by:
//!
//! - the **MTP3** network layer (native, riding M2PA links), and
//! - **M3UA** (RFC 4666), the IP adaptation of the same service.
//!
//! Because a user is generic over the trait, SCCP runs unchanged over either —
//! and a [composite router](Mtp3UserPart) that dispatches by destination can
//! itself implement the trait, unifying several providers behind one SAP.
//!
//! This crate is pure and transport-independent (no SCTP, no async runtime of
//! its own) so it stays portable and testable; the providers pull in the
//! Linux-only SCTP transport behind their own feature flags.
//!
//! ```
//! use ss7_mtp3::{Mtp3Msu, ServiceIndicator, NetworkIndicator, PointCode, Variant};
//!
//! let msu = Mtp3Msu {
//!     si: ServiceIndicator::SCCP,
//!     ni: NetworkIndicator::International,
//!     mp: 0,
//!     opc: PointCode::parse("2-1-3", Variant::Itu).unwrap(),
//!     dpc: PointCode::parse("2-1-4", Variant::Itu).unwrap(),
//!     sls: 0,
//!     data: vec![],
//! };
//! assert_eq!(msu.si, ServiceIndicator::SCCP);
//! ```

pub mod point_code;
pub mod sap;

pub use point_code::{PointCode, PointCodeError, Variant};
pub use sap::{
    Mtp3Error, Mtp3Event, Mtp3Msu, Mtp3Status, Mtp3UserPart, NetworkIndicator, ServiceIndicator,
};
