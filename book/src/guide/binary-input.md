# Binary input

A grammar that starts with `#![input = bytes]` parses `&[u8]` instead of `&str`:

```rust
mod records {
    prpr::grammar! {
        #![input = bytes]

        // A header, then records of a NUL-terminated key and a little-endian `u16`.
        pub records: Vec<(&'i [u8], u16)> = header rs:record* { rs }

        header = b"REC\x01"

        record: (&'i [u8], u16) = k:key v:u16_le { (k, v) }

        key: &'i [u8] = s:([^ '\0']*) b'\0' { #s }

        u16_le: u16 = b:(. .) { u16::from_le_bytes([b.0, b.1]) }
    }
}

fn main() {
    let data = b"REC\x01ab\0\x01\x02\xff\0\x00\x01";
    assert_eq!(
        records::records(data),
        Ok(vec![(&b"ab"[..], 0x0201), (&b"\xff"[..], 0x0100)])
    );
    assert_eq!(
        records::records(b"REC\x02").unwrap_err().to_string(),
        r#"expected `b"REC\x01"`, found `b'R'`"#
    );
}
```

With byte input:

* `.` matches any byte, and its value is a `u8`.
* Character classes match bytes, and their values are `u8`s.
  Their items are characters up to `'\u{ff}'` or byte literals, e.g. `[b'\x80'-b'\xff']`.
* Byte strings, such as `b"REC\x01"`, and byte literals, such as `b'\0'`, are literals, whose values are `&'static [u8]`.
  String literals match their UTF-8 encoding and keep `&'static str` values.
* `#x` is a `&'i [u8]`.
* There is no implicit whitespace and there are no word characters, so tokens are matched as they are and need no delimiters.
  A grammar for a binary format that does have whitespace or words can define `whitespace` and `word_char`, as described in [Whitespace, tokens and delimiters](whitespace.md).

Formats where an earlier field gives the length of a later one cannot be described yet: that needs rules with parameters, which are planned.
