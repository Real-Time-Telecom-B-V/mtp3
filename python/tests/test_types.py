"""Parity / round-trip tests for the mtp3 wheel.

These exercise the same Rust point-code and SAP types the crate ships, through
the Python surface: ``PointCode`` parses/renders the structured form, the
variants carry the right widths, and ``Mtp3Msu`` holds the MTP-TRANSFER fields.
"""

from __future__ import annotations

import pytest

import mtp3


def test_variant_widths() -> None:
    assert mtp3.Variant.Itu.bits() == 14
    assert mtp3.Variant.Itu.octets() == 2
    assert mtp3.Variant.Itu.max_value() == 0x3FFF
    assert mtp3.Variant.Itu.widths() == [3, 8, 3]
    assert mtp3.Variant.Ansi.bits() == 24
    assert mtp3.Variant.Ansi.octets() == 3
    assert mtp3.Variant.Ansi.max_value() == 0xFFFFFF
    assert mtp3.Variant.China.widths() == [8, 8, 8]


def test_point_code_itu_components_roundtrip() -> None:
    # 2-1-3 ITU = (2<<11)|(1<<3)|3 = 4107.
    pc = mtp3.PointCode.from_components((2, 1, 3), mtp3.Variant.Itu)
    assert pc.value() == 4107
    assert pc.components() == [2, 1, 3]
    assert str(pc) == "2-1-3"
    assert pc.variant == mtp3.Variant.Itu


def test_point_code_parse_string_roundtrip() -> None:
    pc = mtp3.PointCode.parse("2-1-3", mtp3.Variant.Itu)
    # parse -> str -> parse is stable
    again = mtp3.PointCode.parse(str(pc), mtp3.Variant.Itu)
    assert pc == again
    assert hash(pc) == hash(again)


def test_point_code_decimal_matches_structured() -> None:
    a = mtp3.PointCode.parse("5687", mtp3.Variant.Itu)
    assert a.value() == 5687
    b = mtp3.PointCode.parse(str(a), mtp3.Variant.Itu)
    assert a == b


def test_point_code_ansi_888_layout() -> None:
    pc = mtp3.PointCode.parse("1-1-5", mtp3.Variant.Ansi)
    assert pc.value() == 65797  # (1<<16)|(1<<8)|5
    assert pc.components() == [1, 1, 5]


def test_point_code_from_value() -> None:
    pc = mtp3.PointCode.from_value(4107, mtp3.Variant.Itu)
    assert pc.components() == [2, 1, 3]


def test_point_code_out_of_range_raises() -> None:
    with pytest.raises(mtp3.Mtp3Error):
        mtp3.PointCode.from_value(0x4000, mtp3.Variant.Itu)  # ITU max is 0x3FFF


def test_point_code_garbage_raises() -> None:
    with pytest.raises(mtp3.Mtp3Error):
        mtp3.PointCode.parse("nope", mtp3.Variant.Itu)
    with pytest.raises(mtp3.Mtp3Error):
        mtp3.PointCode.parse("1-2", mtp3.Variant.Itu)


def test_service_indicator_constants_and_name() -> None:
    assert int(mtp3.ServiceIndicator.SCCP) == 3
    assert mtp3.ServiceIndicator.SCCP.name() == "SCCP"
    assert mtp3.ServiceIndicator.ISUP.value == 5
    assert mtp3.ServiceIndicator(9).name() is None
    assert mtp3.ServiceIndicator(3) == mtp3.ServiceIndicator.SCCP


def test_network_indicator_bits() -> None:
    assert int(mtp3.NetworkIndicator.International) == 0
    assert int(mtp3.NetworkIndicator.National) == 2
    assert mtp3.NetworkIndicator.National.bits() == 2
    assert mtp3.NetworkIndicator.from_bits(2) == mtp3.NetworkIndicator.National


def test_build_mtp3_msu() -> None:
    msu = mtp3.Mtp3Msu(
        mtp3.ServiceIndicator.SCCP,
        mtp3.NetworkIndicator.International,
        mtp3.PointCode.parse("2-1-3", mtp3.Variant.Itu),
        mtp3.PointCode.parse("2-1-4", mtp3.Variant.Itu),
        sls=5,
        data=bytes([0x09, 0x80, 0x03]),
    )
    assert msu.si == mtp3.ServiceIndicator.SCCP
    assert msu.ni == mtp3.NetworkIndicator.International
    assert str(msu.dpc) == "2-1-4"
    assert msu.sls == 5
    assert msu.mp == 0
    assert msu.data == bytes([0x09, 0x80, 0x03])


def test_mtp3_msu_defaults_and_priority_mask() -> None:
    pc = mtp3.PointCode.from_value(1, mtp3.Variant.Itu)
    msu = mtp3.Mtp3Msu(
        mtp3.ServiceIndicator.ISUP,
        mtp3.NetworkIndicator.National,
        pc,
        pc,
        mp=0xFF,  # masked to two bits
    )
    assert msu.mp == 3
    assert msu.sls == 0
    assert msu.data == b""


def test_mtp3_msu_mutable_fields() -> None:
    pc = mtp3.PointCode.from_value(1, mtp3.Variant.Itu)
    msu = mtp3.Mtp3Msu(
        mtp3.ServiceIndicator.SCCP,
        mtp3.NetworkIndicator.International,
        pc,
        pc,
    )
    msu.sls = 7
    msu.data = b"\x01\x02"
    msu.dpc = mtp3.PointCode.from_value(2, mtp3.Variant.Itu)
    assert msu.sls == 7
    assert msu.data == b"\x01\x02"
    assert msu.dpc.value() == 2
