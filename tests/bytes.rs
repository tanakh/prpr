// Byte input.

mod records {
    prpr::grammar! {
        #![input = bytes]

        pub records: Vec<(&'i [u8], u16)> = magic rs:record* { rs }
        magic = b"REC\x01"
        record: (&'i [u8], u16) = k:cstr v:u16le { (k, v) }
        cstr: &'i [u8] = s:([^ '\0']*) b'\0' { #s }
        u16le: u16 = b:(. .) { u16::from_le_bytes([b.0, b.1]) }

        pub any_byte: u8 = .
        pub high: u8 = [b'\x80'-b'\xff']
        pub word = "abc"
    }
}

#[test]
fn records() {
    let data = b"REC\x01ab\0\x01\x02\xff\0\x00\x01";
    assert_eq!(
        records::records(data),
        Ok(vec![(&b"ab"[..], 0x0201), (&b"\xff"[..], 0x0100)])
    );
    let e = records::records(b"REC\x02").unwrap_err();
    assert_eq!(e.to_string(), "expected `b\"REC\\x01\"`, found `b'R'`");
    let e = records::records(b"REC\x01ab\0\x01").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected any character, found end of input".into(), 8)
    );
}

#[test]
fn bytes() {
    assert_eq!(records::any_byte(b"\xfe"), Ok(0xfe));
    assert_eq!(records::high(b"\x80"), Ok(0x80));
    assert!(records::high(b"\x7f").is_err());
    // No implicit whitespace or delimiters with byte input.
    assert!(records::word(b" abc").is_err());
}
