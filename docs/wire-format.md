# Wire format

Hand-rolled binary. This file is the spec: any client (Rust, later Python) must match it byte for byte.

## Global rules

- All multi-byte numbers are **little-endian**, fixed width. No varints.
- Every enum is encoded as a **1-byte tag** followed by its payload.
- Tag `0` is **reserved and invalid** in every enum, so zeroed buffers fail loudly.
- **Tags are never reused.** A removed variant retires its tag forever.
- Variable-length data (strings, lists) is prefixed with a `u32` length (byte count for strings, element count for lists).

## Value

| Tag | Variant | Payload |
| --- | --- | --- |
| 0 | reserved | invalid |
| 1 | Null | none |
| 2 | Boolean | 1 byte: `0` = false, `1` = true, anything else is an error |
| 3 | Integer | 8 bytes, `i64` |
| 4 | Float | 8 bytes, `f64::to_bits()` as `u64` |
| 5 | Text | `u32` byte length, then that many UTF-8 bytes |

Decode errors: input ended early, unknown tag, invalid boolean byte, invalid UTF-8.

## To design later

- Record, Schema, Request, Response encodings
- Frame header: `u32` length + `u8` protocol version (+ message type?); maximum frame size enforced before allocating
