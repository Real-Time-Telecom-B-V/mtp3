# Versioning

`ss7-mtp3` follows [Semantic Versioning 2.0.0](https://semver.org/). The public
API — the `Mtp3UserPart` trait, the `Mtp3Msu`/`Mtp3Event`/`Mtp3Status` types, the
`ServiceIndicator`/`NetworkIndicator` types, and `PointCode`/`Variant` — is the
contract.

## Pre-1.0

This crate is **0.x**: it defines a Service Access Point that its first consumers
(`m3ua`, `mtp3`, `sccp`) are still wiring in. Per SemVer, **minor `0.y` bumps may
make breaking changes** while the SAP settles. It goes 1.0 once at least one real
provider (M3UA or MTP3-over-M2PA) and SCCP consume it end to end.

## The git tag is the source of truth

`Cargo.toml`'s `version` matches the release tag; the release workflow's
`verify-version` job refuses to publish if they disagree. Bump `version`, commit,
tag `vX.Y.Z`, push the tag.

## Post-1.0 rule

- **MAJOR** — remove/rename/re-signature a `pub` item, or change documented SAP
  semantics.
- **MINOR** — backward-compatible additions (new methods with defaults, new
  event/status variants gated appropriately, new point-code variants).
- **PATCH** — bug fixes, docs, behaviour-neutral dependency bumps.
