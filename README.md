# mtp3

[![crates.io](https://img.shields.io/crates/v/mtp3.svg)](https://crates.io/crates/mtp3)
[![docs.rs](https://docs.rs/mtp3/badge.svg)](https://docs.rs/mtp3)
[![CI](https://github.com/Real-Time-Telecom-B-V/mtp3/actions/workflows/ci.yml/badge.svg)](https://github.com/Real-Time-Telecom-B-V/mtp3/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The SS7 **MTP3-User Service Access Point** (`Mtp3UserPart`) and point-code types
— the shared seam of an SS7 network stack. **SCCP** and **ISUP** are written
generic over one trait; **MTP3** (native, over M2PA links) and **M3UA**
(RFC 4666) both implement it, so a user runs unchanged over either transport.

Pure and transport-independent: no SCTP, no async runtime of its own, so it
stays portable and every consumer can unit-test against it.

It ships as **both** a Rust crate (`cargo add mtp3`) and a Rust-backed Python
wheel (`pip install mtp3`), built from one source tree and one version. The
Python wheel exposes the value types — `PointCode` / `Variant`,
`ServiceIndicator`, `NetworkIndicator`, and the `Mtp3Msu` SAP-boundary struct —
so tooling and tests can build routing labels without reimplementing point-code
maths. The async provider trait stays in Rust.

```rust
use mtp3::{Mtp3Msu, Mtp3UserPart, PointCode, ServiceIndicator, NetworkIndicator, Variant};

// SCCP is generic over the SAP — it doesn't care whether `mtp3` sits underneath it:
async fn send_sccp(mtp3: &dyn Mtp3UserPart, sccp_bytes: Vec<u8>) -> Result<(), mtp3::Mtp3Error> {
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

```python
import mtp3

opc = mtp3.PointCode.parse("2-1-3", mtp3.Variant.Itu)   # or "515" (decimal)
dpc = mtp3.PointCode.parse("2-1-4", mtp3.Variant.Itu)
msu = mtp3.Mtp3Msu(
    mtp3.ServiceIndicator.SCCP,
    mtp3.NetworkIndicator.International,
    opc, dpc, sls=0, data=b"...",
)
print(opc.value(), opc.components())                     # 4107 [2, 1, 3]
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

## Performance

mtp3 is a types/SAP crate, not a wire codec, so the hot path a consumer hits (an
STP routing table, config load) is point-code parsing and formatting. Single-core,
`cargo bench` ([`benches/pointcode.rs`](benches/pointcode.rs)); indicative numbers:

| Operation | Time |
|---|---|
| `PointCode::parse` — structured `a-b-c` | ~16 ns |
| `PointCode::parse` — plain decimal | ~12 ns |
| `PointCode::components` | < 1 ns |
| `PointCode::to_string` | ~37 ns |

A counting-allocator [leak check](examples/leak_check.rs)
(`./scripts/mem_leak_test.sh`) hammers point-code parse/format and `Mtp3Msu`
construct/clone and asserts **live bytes stay flat** (Δ 0 over millions of
cycles). Both run in CI.

The Python wheel is the same Rust behind PyO3; it is declared `gil_used = false`,
so it loads on free-threaded ("no-GIL") CPython 3.13t / 3.14t.

## Install

```bash
cargo add mtp3          # Rust crate (zero pyo3 in the default build)
pip install mtp3        # Rust-backed Python wheel (the value types)
```

## Development

```bash
cargo test                              # unit + integration + doctests
cargo test --features python            # + the PyO3 binding face
cargo clippy --all-targets -- -D warnings
cargo bench --no-run
./scripts/mem_leak_test.sh              # live-bytes leak check (PASS/FAIL)
cargo deny check                        # advisories, licenses, sources

# Python wheel
maturin develop && pytest python/tests -q
```

## License

MIT — see [LICENSE](LICENSE).
