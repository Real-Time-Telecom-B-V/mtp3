# mtp3 — overview

The shared seam of an SS7 network stack: the **MTP3-User Service Access Point**
(`Mtp3UserPart`) plus the point-code and SIO types it needs. Pure and
transport-independent — no SCTP, no async runtime of its own — so it stays
portable and every consumer can unit-test against it.

## The idea

SCCP (and ISUP) don't care *how* an MSU reaches the network — only that it does,
with a routing label. So they're written against one trait:

```rust
#[async_trait]
pub trait Mtp3UserPart: Send + Sync {
    async fn send(&self, msu: Mtp3Msu) -> Result<(), Mtp3Error>;   // MTP-TRANSFER.req
    async fn recv(&self) -> Result<Mtp3Event, Mtp3Error>;          // transfer + net-mgmt
    fn is_available(&self, dpc: PointCode) -> bool;
}
```

Two things implement it:

| Provider | how |
|---|---|
| **MTP3** (native) | real MTP3 routing riding **M2PA** links |
| **M3UA** (RFC 4666) | the IP adaptation of the same service — `DATA` ⇆ `Transfer`, `DUNA/DAVA/SCON/DUPU` ⇆ `Pause/Resume/Status` |

Because it's a trait, a user is unified for free. And because the trait is
object-safe, a **router that dispatches by destination is itself an
`Mtp3UserPart`** — so an STP that mixes M3UA and M2PA routes still hands SCCP a
single SAP (see `tests/sap.rs`).

## Types

- `Mtp3Msu` — the routing label (OPC/DPC/SLS) + SIO fields (SI/NI/priority) + the
  MTP3-user payload (the MTP-TRANSFER primitive's parameters).
- `Mtp3Event` — inbound: `Transfer` (MTP-TRANSFER.ind) or `Pause`/`Resume`/`Status`.
- `ServiceIndicator` (SCCP=3, ISUP=5, …), `NetworkIndicator` (intl/national).
- `PointCode` / `Variant` — ITU (14-bit `3-8-3`), ANSI/China (24-bit `8-8-8`),
  parse/format/components. (Consolidated here so the whole stack shares one type.)
- `Mtp3Error`.

## Why it's separate + pure

`async-sctp` (the transport under the providers) is Linux-only. Keeping the SAP
and the point-code types here — with **no** transport dependency — means SCCP,
`m3ua`, and `mtp3` all share one vocabulary while staying portable; the
Linux-only SCTP driving lives behind each provider's own feature flag.
