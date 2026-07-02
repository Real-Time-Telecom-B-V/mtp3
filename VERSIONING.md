# Versioning

`mtp3` follows [Semantic Versioning 2.0.0](https://semver.org/). The public
API — the `Mtp3UserPart` trait, the `Mtp3Msu`/`Mtp3Event`/`Mtp3Status` types, the
`ServiceIndicator`/`NetworkIndicator` types, and `PointCode`/`Variant` — is the
contract.

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
