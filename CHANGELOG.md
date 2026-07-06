# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). See
[VERSIONING.md](VERSIONING.md) for the policy.

## [1.1.0]

### Added
- **`Mtp3Msu::encode` / `Mtp3Msu::decode`** — variant-aware on-wire MSU codec
  (SIO octet + routing label + SIF), so the crate that owns the MTP3 message
  format also owns its bytes. **ITU** (Q.704) packs the 32-bit little-endian
  routing label `DPC(14) | OPC(14) << 14 | SLS(4) << 28`; **ANSI / China**
  (T1.111) lay out DPC(3) + OPC(3) + SLS(1) octets, each point code
  least-significant octet first. Byte-exact against known-answer vectors.
- `Mtp3Msu.encode(variant)` / `Mtp3Msu.decode(data, variant)` on the Python
  `Mtp3Msu`.
- **`Mtp3Error::Decode`** — surfaced when `decode` gets an input too short for
  the SIO + routing label.

## [1.0.1]

### Changed
- Docs: neutral wording for the default point-code variant. No API or behaviour
  change.

## [1.0.0]

First release — the MTP3-User Service Access Point for the SS7 stack.

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

[1.1.0]: https://github.com/Real-Time-Telecom-B-V/mtp3/releases/tag/v1.1.0
[1.0.1]: https://github.com/Real-Time-Telecom-B-V/mtp3/releases/tag/v1.0.1
[1.0.0]: https://github.com/Real-Time-Telecom-B-V/mtp3/releases/tag/v1.0.0
