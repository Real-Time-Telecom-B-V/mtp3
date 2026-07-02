# ss7-mtp3

[![crates.io](https://img.shields.io/crates/v/ss7-mtp3.svg)](https://crates.io/crates/ss7-mtp3)
[![docs.rs](https://docs.rs/ss7-mtp3/badge.svg)](https://docs.rs/ss7-mtp3)
[![CI](https://github.com/Real-Time-Telecom-B-V/ss7-mtp3/actions/workflows/ci.yml/badge.svg)](https://github.com/Real-Time-Telecom-B-V/ss7-mtp3/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The SS7 **MTP3-User Service Access Point** (`Mtp3UserPart`) and point-code types
— the shared seam of an SS7 network stack. **SCCP** and **ISUP** are written
generic over one trait; **MTP3** (native, over M2PA links) and **M3UA**
(RFC 4666) both implement it, so a user runs unchanged over either transport.

Pure and transport-independent: no SCTP, no async runtime of its own, so it
stays portable and every consumer can unit-test against it.

```rust
use ss7_mtp3::{Mtp3Msu, Mtp3UserPart, PointCode, ServiceIndicator, NetworkIndicator, Variant};

// SCCP is generic over the SAP — it doesn't care whether `mtp3` sits underneath it:
async fn send_sccp(mtp3: &dyn Mtp3UserPart, sccp_bytes: Vec<u8>) -> Result<(), ss7_mtp3::Mtp3Error> {
    mtp3.send(Mtp3Msu {
        si: ServiceIndicator::SCCP,
        ni: NetworkIndicator::International,
        mp: 0,
        opc: PointCode::parse("2-1-3", Variant::Itu).unwrap(),
        dpc: PointCode::parse("2-1-4", Variant::Itu).unwrap(),
        sls: 0,
        data: sccp_bytes,
    }).await
}
```

## The SAP

```rust
#[async_trait]
pub trait Mtp3UserPart: Send + Sync {
    async fn send(&self, msu: Mtp3Msu) -> Result<(), Mtp3Error>;   // MTP-TRANSFER.request
    async fn recv(&self) -> Result<Mtp3Event, Mtp3Error>;          // transfer + PAUSE/RESUME/STATUS
    fn is_available(&self, dpc: PointCode) -> bool;
}
```

| Provider | implements it by |
|---|---|
| **MTP3** (native) | real MTP3 routing over **M2PA** links |
| **M3UA** (RFC 4666) | `DATA` ⇆ `Transfer`; `DUNA/DAVA/SCON/DUPU` ⇆ `Pause/Resume/Status` |

The trait is **object-safe**, so a router that dispatches by destination is
itself an `Mtp3UserPart` — an STP mixing M3UA and M2PA routes still hands SCCP a
single SAP, and both providers can serve one SCCP at once. See
[`tests/sap.rs`](tests/sap.rs).

## Types

- `Mtp3Msu` — routing label (OPC/DPC/SLS) + SIO (SI/NI/priority) + user payload.
- `Mtp3Event` — `Transfer` (MTP-TRANSFER.ind) / `Pause` / `Resume` / `Status`.
- `ServiceIndicator` (SCCP=3, ISUP=5, …), `NetworkIndicator` (intl/national).
- `PointCode` / `Variant` — ITU (14-bit), ANSI/China (24-bit); parse, format,
  components.

## Where it fits

```
 sccp / isup  ──generic over──▶  Mtp3UserPart   (this crate; pure, portable)
                                    ▲       ▲
                       mtp3+m2pa ───┘       └─── m3ua        (providers; Linux,
                            └─────── async-sctp ──────┘       feature-gated)
```

More: [`docs/OVERVIEW.md`](docs/OVERVIEW.md).

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo deny check
```

## License

MIT — see [LICENSE](LICENSE).
