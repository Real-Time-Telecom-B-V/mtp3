//! Known-answer vectors for the management and test message codec.
//!
//! Every expected octet string is written out by hand from the figures of ITU-T
//! Q.704 clause 15, Q.2210 9.8.1 and Q.707 5.4: the heading octet is `H0` in the
//! low half and `H1` in the high half, and every multi-octet field is sent least
//! significant octet first. None of them comes from this crate's encoder.

use mtp3::mgmt::{ManagementError, ManagementMessage as M, TestMessage};
use mtp3::{PointCode, Variant};

fn pc(value: u32) -> PointCode {
    PointCode::from_value(value, Variant::Itu).unwrap()
}

/// One message and the octets Q.704 has for it.
fn vectors() -> Vec<(M, Vec<u8>)> {
    vec![
        // 15.4: changeover, a 7-bit sequence number and a filler bit 0.
        (M::Coo { fsn: 0x55 }, vec![0x11, 0x55]),
        (M::Coa { fsn: 0x7F }, vec![0x21, 0x7F]),
        // Q.2210 9.8.1: the extended signals, a 24-bit sequence number.
        (M::Xco { fsn: 0x12_3456 }, vec![0x31, 0x56, 0x34, 0x12]),
        (M::Xca { fsn: 0xFF_FFFF }, vec![0x41, 0xFF, 0xFF, 0xFF]),
        // 15.5: changeback, an 8-bit changeback code.
        (M::Cbd { code: 0xA5 }, vec![0x51, 0xA5]),
        (M::Cba { code: 0xA5 }, vec![0x61, 0xA5]),
        // 15.6: emergency changeover, heading only.
        (M::Eco, vec![0x12]),
        (M::Eca, vec![0x22]),
        // 15.16 and 15.15: congestion test, transfer-controlled with the two
        // spare bits as the congestion status.
        (M::Rct, vec![0x13]),
        (
            M::Tfc {
                destination: pc(2000),
                status: 2,
            },
            vec![0x23, 0xD0, 0x87],
        ),
        // 15.7 to 15.9: destination 2000 is 0x07D0, 14 bits, two spare bits 00.
        (
            M::Tfp {
                destination: pc(2000),
            },
            vec![0x14, 0xD0, 0x07],
        ),
        (
            M::Tfr {
                destination: pc(2000),
            },
            vec![0x34, 0xD0, 0x07],
        ),
        (
            M::Tfa {
                destination: pc(16383),
            },
            vec![0x54, 0xFF, 0x3F],
        ),
        // 15.10: route set test.
        (
            M::Rst {
                destination: pc(2000),
            },
            vec![0x15, 0xD0, 0x07],
        ),
        (M::Rsr { destination: pc(1) }, vec![0x25, 0x01, 0x00]),
        // 15.11: management inhibit, heading only.
        (M::Lin, vec![0x16]),
        (M::Lun, vec![0x26]),
        (M::Lia, vec![0x36]),
        (M::Lua, vec![0x46]),
        (M::Lid, vec![0x56]),
        (M::Lfu, vec![0x66]),
        (M::Llt, vec![0x76]),
        (M::Lrt, vec![0x86]),
        // 15.12: traffic restart allowed.
        (M::Tra, vec![0x17]),
        // 15.13 and 15.14: data link connection, a 12-bit identity, 4 spare bits.
        (M::Dlc { link: 0xABC }, vec![0x18, 0xBC, 0x0A]),
        (M::Css, vec![0x28]),
        (M::Cns, vec![0x38]),
        (M::Cnp, vec![0x48]),
        // 15.17: user part unavailable, SCCP (3), cause 1 (unequipped).
        (
            M::Upu {
                destination: pc(2000),
                user_part: 3,
                cause: 1,
            },
            vec![0x1A, 0xD0, 0x07, 0x13],
        ),
    ]
}

#[test]
fn every_management_message_encodes_to_the_octets_of_the_figure() {
    for (message, octets) in vectors() {
        assert_eq!(message.encode(Variant::Itu).unwrap(), octets, "{message:?}");
    }
}

#[test]
fn every_management_message_is_read_from_the_octets_of_the_figure() {
    for (message, octets) in vectors() {
        assert_eq!(
            M::decode(&octets, Variant::Itu).unwrap(),
            message,
            "{octets:02x?}"
        );
    }
}

#[test]
fn the_heading_codes_are_those_of_table_1() {
    assert_eq!(M::Coo { fsn: 0 }.heading(), (1, 1));
    assert_eq!(M::Tfp { destination: pc(1) }.heading(), (4, 1));
    assert_eq!(M::Tfa { destination: pc(1) }.heading(), (4, 5));
    assert_eq!(M::Tra.heading(), (7, 1));
    assert_eq!(M::Tra.name(), "TRA");
    assert_eq!(M::Xco { fsn: 0 }.name(), "XCO");
}

#[test]
fn bits_outside_a_field_do_not_reach_the_wire_or_the_value() {
    // The filler bit of a changeover order and the spare bits of a destination
    // are 0 when sent, whatever the value handed in, and ignored when received.
    assert_eq!(
        M::Coo { fsn: 0xFF }.encode(Variant::Itu).unwrap(),
        [0x11, 0x7F]
    );
    assert_eq!(
        M::decode(&[0x11, 0xD5], Variant::Itu).unwrap(),
        M::Coo { fsn: 0x55 }
    );
    assert_eq!(
        M::decode(&[0x14, 0xD0, 0xC7], Variant::Itu).unwrap(),
        M::Tfp {
            destination: pc(2000)
        }
    );
    assert_eq!(
        M::Xco { fsn: 0x0123_4567 }.encode(Variant::Itu).unwrap(),
        [0x31, 0x67, 0x45, 0x23]
    );
}

#[test]
fn octets_after_the_last_field_are_ignored() {
    // A national option may add octets; the fields this codec knows still read.
    assert_eq!(
        M::decode(&[0x17, 0xAA, 0xBB], Variant::Itu).unwrap(),
        M::Tra
    );
}

#[test]
fn malformed_management_octets_are_errors_and_never_panic() {
    assert_eq!(
        M::decode(&[], Variant::Itu),
        Err(ManagementError::Truncated)
    );
    // Every heading that carries a field, cut before the field ends.
    for cut in [
        &[0x11][..],
        &[0x31, 0x01, 0x02],
        &[0x51],
        &[0x23, 0xD0],
        &[0x14],
        &[0x14, 0xD0],
        &[0x15, 0xD0],
        &[0x18, 0xBC],
        &[0x1A, 0xD0, 0x07],
    ] {
        assert_eq!(
            M::decode(cut, Variant::Itu),
            Err(ManagementError::Truncated),
            "{cut:02x?}"
        );
    }
    // Spare headings of Table 1/Q.704, the two marked "should not be used" among them.
    for spare in [
        0x00u8, 0x71, 0x24, 0x44, 0x35, 0x96, 0x27, 0x58, 0x19, 0x2A, 0x1F,
    ] {
        assert_eq!(
            M::decode(&[spare, 0, 0, 0], Variant::Itu),
            Err(ManagementError::UnknownHeading {
                h0: spare & 0x0F,
                h1: spare >> 4
            })
        );
    }
    // Every one- and two-octet input is an answer, not a panic.
    for a in 0..=255u8 {
        let _ = M::decode(&[a], Variant::Itu);
        for b in [0x00u8, 0x7F, 0xFF] {
            let _ = M::decode(&[a, b], Variant::Itu);
            let _ = TestMessage::decode(&[a, b], Variant::Itu);
        }
    }
}

#[test]
fn a_variant_whose_formats_are_not_implemented_is_refused() {
    for variant in [Variant::Ansi, Variant::China] {
        assert_eq!(
            M::Tra.encode(variant),
            Err(ManagementError::UnsupportedVariant(variant))
        );
        assert_eq!(
            M::decode(&[0x17], variant),
            Err(ManagementError::UnsupportedVariant(variant))
        );
        assert_eq!(
            TestMessage::decode(&[0x11, 0x00], variant),
            Err(ManagementError::UnsupportedVariant(variant))
        );
    }
    // A destination of another variant does not fit the 14-bit field.
    let ansi = PointCode::from_value(0x01_0203, Variant::Ansi).unwrap();
    assert_eq!(
        M::Tfp { destination: ansi }.encode(Variant::Itu),
        Err(ManagementError::UnsupportedVariant(Variant::Ansi))
    );
}

// ── Q.707 5.4: signalling link test ──────────────────────────────────────────

#[test]
fn a_link_test_message_is_heading_length_and_pattern() {
    // H0 0001, H1 0001 (SLTM) or 0010 (SLTA); four spare bits, then the length
    // in the high half of the octet; then the pattern.
    let pattern = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let sltm = TestMessage::Sltm {
        pattern: pattern.clone(),
    };
    let slta = TestMessage::Slta {
        pattern: pattern.clone(),
    };
    assert_eq!(
        sltm.encode(Variant::Itu).unwrap(),
        [0x11, 0x40, 0xDE, 0xAD, 0xBE, 0xEF]
    );
    assert_eq!(
        slta.encode(Variant::Itu).unwrap(),
        [0x21, 0x40, 0xDE, 0xAD, 0xBE, 0xEF]
    );
    assert_eq!(
        TestMessage::decode(&[0x11, 0x40, 0xDE, 0xAD, 0xBE, 0xEF], Variant::Itu).unwrap(),
        sltm
    );
    assert_eq!(
        TestMessage::decode(&[0x21, 0x40, 0xDE, 0xAD, 0xBE, 0xEF], Variant::Itu).unwrap(),
        slta
    );
    assert_eq!(sltm.pattern(), &pattern[..]);
    assert_eq!(sltm.name(), "SLTM");
}

#[test]
fn a_test_pattern_is_one_to_fifteen_octets() {
    // Q.707 5.4: "Test pattern: (n x 8 bits, 1 <= n <= 15)".
    let longest = TestMessage::Sltm {
        pattern: (1..=15).collect(),
    };
    let wire = longest.encode(Variant::Itu).unwrap();
    assert_eq!(wire[..2], [0x11, 0xF0]);
    assert_eq!(TestMessage::decode(&wire, Variant::Itu).unwrap(), longest);

    for bad in [0usize, 16] {
        assert_eq!(
            TestMessage::Sltm {
                pattern: vec![0; bad]
            }
            .encode(Variant::Itu),
            Err(ManagementError::PatternLength(bad))
        );
    }
    // Received with length 0: outside the range, refused.
    assert_eq!(
        TestMessage::decode(&[0x11, 0x00], Variant::Itu),
        Err(ManagementError::PatternLength(0))
    );
}

#[test]
fn malformed_link_test_octets_are_errors() {
    assert_eq!(
        TestMessage::decode(&[], Variant::Itu),
        Err(ManagementError::Truncated)
    );
    assert_eq!(
        TestMessage::decode(&[0x11], Variant::Itu),
        Err(ManagementError::Truncated)
    );
    // The length says four octets, two are there.
    assert_eq!(
        TestMessage::decode(&[0x11, 0x40, 0xDE, 0xAD], Variant::Itu),
        Err(ManagementError::Truncated)
    );
    assert_eq!(
        TestMessage::decode(&[0x31, 0x10, 0xAA], Variant::Itu),
        Err(ManagementError::UnknownHeading { h0: 1, h1: 3 })
    );
    // The spare bits are ignored on receipt; octets after the pattern too.
    assert_eq!(
        TestMessage::decode(&[0x21, 0x1F, 0xAA, 0xBB], Variant::Itu).unwrap(),
        TestMessage::Slta {
            pattern: vec![0xAA]
        }
    );
}
