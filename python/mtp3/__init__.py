"""mtp3 — Rust-backed SS7 MTP3-User SAP types + point codes for Python.

The MTP3-User Service Access Point is the seam SCCP and ISUP are written against;
MTP3 (native, over M2PA links) and M3UA (RFC 4666) both implement it. This
package exposes the crate's value types — point codes and the SIO/routing-label
fields of the MTP-TRANSFER primitive — from one source tree / one version.

This is a *types* package, not a wire codec: ``Mtp3Msu`` is the SAP-boundary data
holder, and the async provider trait (``Mtp3UserPart``) stays in Rust.
"""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version

from ._mtp3 import (
    Mtp3Error,
    Mtp3Msu,
    NetworkIndicator,
    PointCode,
    ServiceIndicator,
    Variant,
)

try:
    __version__ = version("mtp3")
except PackageNotFoundError:  # running from a source checkout without an installed dist
    __version__ = "0.0.0+unknown"

__all__ = [
    # point codes
    "PointCode",
    "Variant",
    # SIO fields
    "ServiceIndicator",
    "NetworkIndicator",
    # SAP-boundary MSU
    "Mtp3Msu",
    # errors
    "Mtp3Error",
    "__version__",
]
