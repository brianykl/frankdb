

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
    todo!()
}

pub fn decode_value(input: &mut &[u8]) -> Result<Value, WireError> {
    todo!()
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], WireError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
}