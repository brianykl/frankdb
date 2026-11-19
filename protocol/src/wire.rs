

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
}