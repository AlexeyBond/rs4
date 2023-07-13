use std::ops::Neg;
use std::str;

fn try_parse(source: &[u8], radix: u32) -> Option<u16> {
    match source[0] {
        b'-' => {
            let absolute = u16::from_str_radix(str::from_utf8(&source[1..]).ok()?, radix).ok()?;
            let signed = i16::try_from(absolute).ok()?.neg();
            let unsigned_repr = signed as u16;

            Some(unsigned_repr)
        }
        _ => u16::from_str_radix(str::from_utf8(source).ok()?, radix).ok()
    }
}

/// Try to parse a numeric literal.
///
/// See: https://forth-standard.org/standard/usage#usage:numbers
pub fn parse_literal(source: &[u8], default_radix: u32) -> Option<u16> {
    if source.len() == 0 {
        return None;
    }

    match source[0] {
        b'#' => try_parse(&source[1..], 10),
        b'$' => try_parse(&source[1..], 16),
        b'%' => try_parse(&source[1..], 2),
        _ => try_parse(source, default_radix),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_unsigned() {
        assert_eq!(
            parse_literal(b"10050", 10),
            Some(10050),
        );
        assert_eq!(
            parse_literal(b"+10050", 10),
            Some(10050),
        );

        assert_eq!(
            parse_literal(b"$FFFF", 10),
            Some(0xFFFF),
        );

        assert_eq!(
            parse_literal(b"%1111000011110000", 10),
            Some(0b1111_0000_1111_0000),
        )
    }

    fn assert_parse_negative(src: &[u8], default_radix: u32, expected_abs: u16) {
        assert_eq!(
            parse_literal(src, default_radix).unwrap(),
            0u16.wrapping_sub(expected_abs)
        )
    }

    #[test]
    fn test_parse_signed() {
        assert_parse_negative(b"-1", 10, 1);
        assert_parse_negative(b"$-7FFF", 10, 0x7FFF);
        assert_parse_negative(b"%-10101010", 10, 0b10101010);
    }

    #[test]
    fn test_parse_different_radix() {
        assert_eq!(
            parse_literal(b"zZz", 36).unwrap(),
            46655
        )
    }

    #[test]
    fn test_parse_overflow() {
        assert_eq!(
            parse_literal(b"100500", 10),
            None
        );

        assert_eq!(
            parse_literal(b"$-8FFF", 10),
            None
        )
    }

    #[test]
    fn test_parse_bad_string() {
        assert_eq!(
            parse_literal(b"Z", 10),
            None
        )
    }
}

