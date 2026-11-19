

use crate::value::Value;

#[derive(Debug, PartialEq)]
pub enum WireError {
    UnexpectedEnd,
    UnknownTag(u8),
    InvalidBoolean(u8),
    InvalidUtf8,
}

const TAG_NULL: u8 = 1;
const TAG_BOOLEAN: u8 = 2;
const TAG_INTEGER: u8 = 3;
const TAG_FLOAT: u8 = 4;
const TAG_TEXT: u8 = 5;

pub fn encode_value(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.push(TAG_NULL),
        Value::Boolean(b) => {
            out.push(TAG_BOOLEAN);
            out.push(*b as u8);
        }
        Value::Integer(i) => {
            out.push(TAG_INTEGER);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Value::Float(f) => {
            out.push(TAG_FLOAT);
            out.extend_from_slice(&f.to_bits().to_le_bytes());
        }
        Value::Text(s) => {
            out.push(TAG_TEXT);
            let len = u32::try_from(s.len()).expect("text longer than u32::MAX bytes");
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
    }
}

pub fn decode_value(input: &mut &[u8]) -> Result<Value, WireError> {
    let tag = take(input, 1)?[0];
    match tag {
        TAG_NULL => Ok(Value::Null),
        TAG_BOOLEAN => match take(input, 1)?[0] {
            0 => Ok(Value::Boolean(false)),
            1 => Ok(Value::Boolean(true)),
            other => Err(WireError::InvalidBoolean(other)),
        },
        TAG_INTEGER => {
            let bytes = take(input, 8)?;
            let array: [u8; 8] = bytes.try_into().expect("take returned 8 bytes");
            Ok(Value::Integer(i64::from_le_bytes(array)))
        }
        TAG_FLOAT => {
            let bytes = take(input, 8)?;
            let array: [u8; 8] = bytes.try_into().expect("take returned 8 bytes");
            Ok(Value::Float(f64::from_bits(u64::from_le_bytes(array))))
        }
        TAG_TEXT => {
            let len_bytes = take(input, 4)?;
            let len_array: [u8; 4] = len_bytes.try_into().expect("take returned 4 bytes");
            let len = u32::from_le_bytes(len_array) as usize;
            let text_bytes = take(input, len)?;
            let text = std::str::from_utf8(text_bytes).map_err(|_| WireError::InvalidUtf8)?;
            Ok(Value::Text(text.to_string()))
        }
        other => Err(WireError::UnknownTag(other)),
    }
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], WireError> {
    let bytes: &'a [u8] = *input;
    if bytes.len() < n {
        return Err(WireError::UnexpectedEnd);
    }

    let (head, rest) = bytes.split_at(n);
    *input = rest;
    Ok(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(value: &Value) -> Vec<u8> {
        let mut out = Vec::new();
        encode_value(value, &mut out);
        out
    }

    #[test]
    fn encodes_null() {
        assert_eq!(encode(&Value::Null), vec![1]);
    }

    #[test]
    fn encodes_booleans() {
        assert_eq!(encode(&Value::Boolean(true)), vec![2, 1]);
        assert_eq!(encode(&Value::Boolean(false)), vec![2, 0]);
    }

    #[test]
    fn encodes_integer_little_endian() {
        assert_eq!(encode(&Value::Integer(5)), vec![3, 5, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encodes_negative_integer() {
        assert_eq!(
            encode(&Value::Integer(-1)),
            vec![3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn encodes_float_as_raw_bits() {
        // 1.5 as f64 bits = 0x3FF8_0000_0000_0000
        assert_eq!(encode(&Value::Float(1.5)), vec![4, 0, 0, 0, 0, 0, 0, 0xF8, 0x3F]);
    }

    #[test]
    fn encodes_text_with_length_prefix() {
        assert_eq!(
            encode(&Value::Text("hi".to_string())),
            vec![5, 2, 0, 0, 0, b'h', b'i']
        );
    }

    #[test]
    fn text_length_counts_bytes_not_chars() {
        // "é" is one character but two UTF-8 bytes: 0xC3 0xA9
        assert_eq!(
            encode(&Value::Text("é".to_string())),
            vec![5, 2, 0, 0, 0, 0xC3, 0xA9]
        );
    }

    #[test]
    fn empty_text_has_zero_length() {
        assert_eq!(encode(&Value::Text(String::new())), vec![5, 0, 0, 0, 0]);
    }

    #[test]
    fn appends_without_clearing_existing_bytes() {
        let mut out = vec![0xAA];
        encode_value(&Value::Null, &mut out);
        assert_eq!(out, vec![0xAA, 1]);
    }

    fn decode_all(bytes: &[u8]) -> Result<Value, WireError> {
        let mut input = bytes;
        let value = decode_value(&mut input)?;
        assert!(input.is_empty(), "decoder left {} bytes", input.len());
        Ok(value)
    }

    fn round_trip(value: Value) {
        assert_eq!(decode_all(&encode(&value)), Ok(value));
    }

    fn float_round_trip_bits(f: f64) {
        match decode_all(&encode(&Value::Float(f))) {
            Ok(Value::Float(decoded)) => assert_eq!(decoded.to_bits(), f.to_bits()),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    #[test]
    fn round_trips_null_and_booleans() {
        round_trip(Value::Null);
        round_trip(Value::Boolean(true));
        round_trip(Value::Boolean(false));
    }

    #[test]
    fn round_trips_integers() {
        round_trip(Value::Integer(0));
        round_trip(Value::Integer(-1));
        round_trip(Value::Integer(i64::MIN));
        round_trip(Value::Integer(i64::MAX));
    }

    #[test]
    fn round_trips_text() {
        round_trip(Value::Text(String::new()));
        round_trip(Value::Text("hi".to_string()));
        round_trip(Value::Text("é".to_string()));
    }

    #[test]
    fn round_trips_floats_bit_exact() {
        // Compare bits: -0.0 == 0.0 and NaN != NaN under PartialEq.
        float_round_trip_bits(1.5);
        float_round_trip_bits(-0.0);
        float_round_trip_bits(f64::NAN);
        float_round_trip_bits(f64::INFINITY);
    }

    #[test]
    fn empty_input_is_unexpected_end() {
        assert_eq!(decode_all(&[]), Err(WireError::UnexpectedEnd));
    }

    #[test]
    fn truncated_integer_is_unexpected_end() {
        assert_eq!(decode_all(&[3, 1, 2, 3]), Err(WireError::UnexpectedEnd));
    }

    #[test]
    fn truncated_text_length_is_unexpected_end() {
        assert_eq!(decode_all(&[5, 2, 0]), Err(WireError::UnexpectedEnd));
    }

    #[test]
    fn text_shorter_than_its_length_is_unexpected_end() {
        assert_eq!(
            decode_all(&[5, 10, 0, 0, 0, b'h', b'i']),
            Err(WireError::UnexpectedEnd)
        );
    }

    #[test]
    fn huge_text_length_fails_without_allocating() {
        assert_eq!(
            decode_all(&[5, 0xFF, 0xFF, 0xFF, 0xFF]),
            Err(WireError::UnexpectedEnd)
        );
    }

    #[test]
    fn missing_boolean_byte_is_unexpected_end() {
        assert_eq!(decode_all(&[2]), Err(WireError::UnexpectedEnd));
    }

    #[test]
    fn reserved_tag_zero_is_rejected() {
        assert_eq!(decode_all(&[0]), Err(WireError::UnknownTag(0)));
    }

    #[test]
    fn unknown_tag_is_rejected() {
        assert_eq!(decode_all(&[99]), Err(WireError::UnknownTag(99)));
    }

    #[test]
    fn invalid_boolean_byte_is_rejected() {
        assert_eq!(decode_all(&[2, 2]), Err(WireError::InvalidBoolean(2)));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        assert_eq!(decode_all(&[5, 1, 0, 0, 0, 0xFF]), Err(WireError::InvalidUtf8));
    }

    #[test]
    fn decodes_consecutive_values_and_advances_input() {
        let mut buf = encode(&Value::Integer(7));
        encode_value(&Value::Text("x".to_string()), &mut buf);
        buf.push(0xAB);

        let mut input = buf.as_slice();
        assert_eq!(decode_value(&mut input), Ok(Value::Integer(7)));
        assert_eq!(decode_value(&mut input), Ok(Value::Text("x".to_string())));
        assert_eq!(input, &[0xAB]);
    }
}