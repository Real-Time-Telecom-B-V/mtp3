# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). See
[VERSIONING.md](VERSIONING.md) for the policy — note this crate is pre-1.0, so
the SAP may still evolve as its first consumers (`m3ua`, `mtp3`, `sccp`) wire in.

## [0.1.0]

First release — the MTP3-User Service Access Point for the RTT SS7 stack.

### Added
- **`Mtp3UserPart`** — the async SAP trait (`send` / `recv` / `is_available`),
  object-safe so a composite router can implement it too.
- **`Mtp3Msu`** — routing label (OPC/DPC/SLS) + SIO fields (SI/NI/priority) +
  MTP3-user payload; **`Mtp3Event`** (`Transfer` / `Pause` / `Resume` / `Status`);
  **`Mtp3Status`**; **`Mtp3Error`**.
- **`ServiceIndicator`** (SCCP/ISUP/… constants) and **`NetworkIndicator`**.
- **`PointCode`** / **`Variant`** — ITU / ANSI / China point codes (parse,
  format, components), consolidated here so the whole stack shares one type.
- Tests incl. an async in-memory provider and the composable-router pattern.

[0.1.0]: https://github.com/Real-Time-Telecom-B-V/ss7-mtp3/releases/tag/v0.1.0
