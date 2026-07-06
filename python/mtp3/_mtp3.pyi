"""Type stubs for the Rust-backed ``mtp3._mtp3`` extension module."""

from __future__ import annotations

class Mtp3Error(Exception):
    """MTP3 point-code / SAP value error."""

class Variant:
    """The SS7 variant — fixes a point code's width and structured layout.

    A PyO3 enum (not a Python ``enum.Enum``): compare members with ``==``.
    """

    Itu: Variant
    Ansi: Variant
    China: Variant
    def bits(self) -> int:
        """Total point-code width in bits (14 for ITU, 24 for ANSI/China)."""
    def octets(self) -> int:
        """Octets a point code occupies on the wire (rounded up)."""
    def max_value(self) -> int:
        """Largest value representable in this variant."""
    def widths(self) -> list[int]:
        """Component bit widths, most-significant first."""
    def name(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class PointCode:
    """An SS7 point code plus the :class:`Variant` that gives it meaning."""

    @property
    def variant(self) -> Variant: ...
    @staticmethod
    def parse(s: str, variant: Variant) -> PointCode:
        """Parse ``"2-1-3"`` (structured) or ``"515"`` (decimal); ``-``/``.`` separators."""
    @staticmethod
    def from_value(value: int, variant: Variant) -> PointCode:
        """Build from a raw integer, validating it fits the variant's width."""
    @staticmethod
    def from_components(parts: tuple[int, int, int], variant: Variant) -> PointCode:
        """Build from three structured components (most-significant first)."""
    def value(self) -> int:
        """The raw integer value (right-aligned in the routing label)."""
    def components(self) -> list[int]:
        """Decompose into the three structured components."""
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class ServiceIndicator:
    """Service Indicator — the low nibble of the SIO (which MTP3-user owns it)."""

    SNM: ServiceIndicator
    SNT: ServiceIndicator
    SCCP: ServiceIndicator
    TUP: ServiceIndicator
    ISUP: ServiceIndicator
    @property
    def value(self) -> int: ...
    def __init__(self, value: int) -> None: ...
    def name(self) -> str | None:
        """The well-known name (e.g. ``"SCCP"``), or ``None``."""
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class NetworkIndicator:
    """Network Indicator — the top two bits of the SIO.

    A PyO3 enum: members compare equal to their on-wire integer (``int(...)``
    yields the wire value).
    """

    International: NetworkIndicator
    InternationalSpare: NetworkIndicator
    National: NetworkIndicator
    NationalSpare: NetworkIndicator
    @staticmethod
    def from_bits(v: int) -> NetworkIndicator:
        """Build from the raw two-bit SIO field (masked to 2 bits)."""
    def bits(self) -> int:
        """The two-bit SIO encoding."""
    def __int__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class Mtp3Msu:
    """An MTP3 MSU at the MTP3-user boundary — routing label + SIO + payload.

    The parameters of the MTP-TRANSFER primitive (Q.701). :meth:`encode` /
    :meth:`decode` render it to and from the on-wire MSU (SIO + routing label +
    SIF) for a given :class:`Variant` — ITU Q.704 or ANSI T1.111.
    """

    si: ServiceIndicator
    ni: NetworkIndicator
    mp: int
    opc: PointCode
    dpc: PointCode
    sls: int
    data: bytes
    def __init__(
        self,
        si: ServiceIndicator,
        ni: NetworkIndicator,
        opc: PointCode,
        dpc: PointCode,
        *,
        mp: int = 0,
        sls: int = 0,
        data: bytes = b"",
    ) -> None: ...
    def encode(self, variant: Variant) -> bytes:
        """Encode to the on-wire MSU bytes (SIO + routing label + SIF)."""
    @staticmethod
    def decode(data: bytes, variant: Variant) -> Mtp3Msu:
        """Decode on-wire MSU bytes; raises ``Mtp3Error`` if too short."""
