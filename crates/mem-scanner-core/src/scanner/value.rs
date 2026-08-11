use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    ByteArray,
}

impl ValueType {
    /// Returns the byte size of this value type.
    /// For ByteArray, returns 0 (variable length).
    pub fn size(&self) -> usize {
        match self {
            ValueType::I8 | ValueType::U8 => 1,
            ValueType::I16 | ValueType::U16 => 2,
            ValueType::I32 | ValueType::U32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::U64 | ValueType::F64 => 8,
            ValueType::ByteArray => 0,
        }
    }
}

/// A byte pattern with optional wildcards (None = wildcard "??")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytePattern(pub Vec<Option<u8>>);

impl BytePattern {
    /// Parse a hex string pattern like "48 8B ?? ?? 90 E8"
    pub fn parse(input: &str) -> Result<Self, String> {
        let bytes = input
            .split_whitespace()
            .map(|s| {
                if s == "??" || s == "?" {
                    Ok(None)
                } else {
                    u8::from_str_radix(s, 16)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if bytes.is_empty() {
            return Err("Empty pattern".into());
        }
        Ok(Self(bytes))
    }

    /// Check if the pattern matches the given data slice.
    pub fn matches(&self, data: &[u8]) -> bool {
        if data.len() < self.0.len() {
            return false;
        }
        self.0
            .iter()
            .zip(data)
            .all(|(pat, &byte)| pat.map_or(true, |p| p == byte))
    }

    /// Length of the pattern in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the pattern is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>),
    Pattern(BytePattern),
}

impl ScanValue {
    /// Serialize the value to little-endian bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ScanValue::I8(v) => v.to_le_bytes().to_vec(),
            ScanValue::I16(v) => v.to_le_bytes().to_vec(),
            ScanValue::I32(v) => v.to_le_bytes().to_vec(),
            ScanValue::I64(v) => v.to_le_bytes().to_vec(),
            ScanValue::U8(v) => v.to_le_bytes().to_vec(),
            ScanValue::U16(v) => v.to_le_bytes().to_vec(),
            ScanValue::U32(v) => v.to_le_bytes().to_vec(),
            ScanValue::U64(v) => v.to_le_bytes().to_vec(),
            ScanValue::F32(v) => v.to_le_bytes().to_vec(),
            ScanValue::F64(v) => v.to_le_bytes().to_vec(),
            ScanValue::Bytes(v) => v.clone(),
            ScanValue::Pattern(p) => {
                // Return only the non-wildcard bytes (used for length estimation)
                p.0.iter().filter_map(|b| *b).collect()
            }
        }
    }

    /// Returns the ValueType corresponding to this value.
    pub fn value_type(&self) -> ValueType {
        match self {
            ScanValue::I8(_) => ValueType::I8,
            ScanValue::I16(_) => ValueType::I16,
            ScanValue::I32(_) => ValueType::I32,
            ScanValue::I64(_) => ValueType::I64,
            ScanValue::U8(_) => ValueType::U8,
            ScanValue::U16(_) => ValueType::U16,
            ScanValue::U32(_) => ValueType::U32,
            ScanValue::U64(_) => ValueType::U64,
            ScanValue::F32(_) => ValueType::F32,
            ScanValue::F64(_) => ValueType::F64,
            ScanValue::Bytes(_) => ValueType::ByteArray,
            ScanValue::Pattern(_) => ValueType::ByteArray,
        }
    }

    /// Compare a raw byte slice against this value with a given condition.
    /// `previous` is the previous raw bytes at that address (for Changed/Increased/etc).
    pub fn matches(
        &self,
        data: &[u8],
        condition: ScanCondition,
        previous: Option<&[u8]>,
    ) -> bool {
        match condition {
            ScanCondition::Exact => {
                match self {
                    ScanValue::Pattern(p) => p.matches(data),
                    _ => data == self.to_bytes().as_slice(),
                }
            }
            ScanCondition::GreaterThan => self.cmp_bytes(data, Ordering::Greater),
            ScanCondition::LessThan => self.cmp_bytes(data, Ordering::Less),
            ScanCondition::Between => true, // handled externally with value2
            ScanCondition::Unknown => true,
            ScanCondition::Changed => {
                if let Some(prev) = previous {
                    data != prev
                } else {
                    false
                }
            }
            ScanCondition::Unchanged => {
                if let Some(prev) = previous {
                    data == prev
                } else {
                    true
                }
            }
            ScanCondition::Increased => {
                if let Some(prev) = previous {
                    self.cmp_raw(data, prev, Ordering::Greater)
                } else {
                    false
                }
            }
            ScanCondition::Decreased => {
                if let Some(prev) = previous {
                    self.cmp_raw(data, prev, Ordering::Less)
                } else {
                    false
                }
            }
            ScanCondition::IncreasedBy => {
                if let Some(prev) = previous {
                    self.check_delta(data, prev, true)
                } else {
                    false
                }
            }
            ScanCondition::DecreasedBy => {
                if let Some(prev) = previous {
                    self.check_delta(data, prev, false)
                } else {
                    false
                }
            }
        }
    }

    /// Compare data bytes against self's value. Returns true if data `ord` self.
    fn cmp_bytes(&self, data: &[u8], ord: Ordering) -> bool {
        macro_rules! cmp_typed {
            ($ty:ty, $variant:ident) => {{
                if let ScanValue::$variant(target) = self {
                    if data.len() < std::mem::size_of::<$ty>() {
                        return false;
                    }
                    let val = <$ty>::from_le_bytes(
                        data[..std::mem::size_of::<$ty>()].try_into().unwrap(),
                    );
                    val.partial_cmp(target) == Some(ord)
                } else {
                    false
                }
            }};
        }
        match self.value_type() {
            ValueType::I8 => cmp_typed!(i8, I8),
            ValueType::I16 => cmp_typed!(i16, I16),
            ValueType::I32 => cmp_typed!(i32, I32),
            ValueType::I64 => cmp_typed!(i64, I64),
            ValueType::U8 => cmp_typed!(u8, U8),
            ValueType::U16 => cmp_typed!(u16, U16),
            ValueType::U32 => cmp_typed!(u32, U32),
            ValueType::U64 => cmp_typed!(u64, U64),
            ValueType::F32 => cmp_typed!(f32, F32),
            ValueType::F64 => cmp_typed!(f64, F64),
            ValueType::ByteArray => false,
        }
    }

    /// Compare two raw byte slices interpreted as the same type.
    /// Returns true if `current` `ord` `previous`.
    fn cmp_raw(&self, current: &[u8], previous: &[u8], ord: Ordering) -> bool {
        macro_rules! cmp_raw_typed {
            ($ty:ty) => {{
                let size = std::mem::size_of::<$ty>();
                if current.len() < size || previous.len() < size {
                    return false;
                }
                let cur = <$ty>::from_le_bytes(current[..size].try_into().unwrap());
                let prev = <$ty>::from_le_bytes(previous[..size].try_into().unwrap());
                cur.partial_cmp(&prev) == Some(ord)
            }};
        }
        match self.value_type() {
            ValueType::I8 => cmp_raw_typed!(i8),
            ValueType::I16 => cmp_raw_typed!(i16),
            ValueType::I32 => cmp_raw_typed!(i32),
            ValueType::I64 => cmp_raw_typed!(i64),
            ValueType::U8 => cmp_raw_typed!(u8),
            ValueType::U16 => cmp_raw_typed!(u16),
            ValueType::U32 => cmp_raw_typed!(u32),
            ValueType::U64 => cmp_raw_typed!(u64),
            ValueType::F32 => cmp_raw_typed!(f32),
            ValueType::F64 => cmp_raw_typed!(f64),
            ValueType::ByteArray => false,
        }
    }

    /// Check if (current - previous) == self (increased_by) or (previous - current) == self (decreased_by).
    fn check_delta(&self, current: &[u8], previous: &[u8], increased: bool) -> bool {
        macro_rules! delta_typed {
            ($ty:ty, $variant:ident) => {{
                if let ScanValue::$variant(delta) = self {
                    let size = std::mem::size_of::<$ty>();
                    if current.len() < size || previous.len() < size {
                        return false;
                    }
                    let cur = <$ty>::from_le_bytes(current[..size].try_into().unwrap());
                    let prev = <$ty>::from_le_bytes(previous[..size].try_into().unwrap());
                    if increased {
                        cur.wrapping_sub(prev) == *delta
                    } else {
                        prev.wrapping_sub(cur) == *delta
                    }
                } else {
                    false
                }
            }};
        }
        macro_rules! delta_float {
            ($ty:ty, $variant:ident) => {{
                if let ScanValue::$variant(delta) = self {
                    let size = std::mem::size_of::<$ty>();
                    if current.len() < size || previous.len() < size {
                        return false;
                    }
                    let cur = <$ty>::from_le_bytes(current[..size].try_into().unwrap());
                    let prev = <$ty>::from_le_bytes(previous[..size].try_into().unwrap());
                    let diff = if increased { cur - prev } else { prev - cur };
                    (diff - *delta).abs() < 0.001 as $ty
                } else {
                    false
                }
            }};
        }
        match self.value_type() {
            ValueType::I8 => delta_typed!(i8, I8),
            ValueType::I16 => delta_typed!(i16, I16),
            ValueType::I32 => delta_typed!(i32, I32),
            ValueType::I64 => delta_typed!(i64, I64),
            ValueType::U8 => delta_typed!(u8, U8),
            ValueType::U16 => delta_typed!(u16, U16),
            ValueType::U32 => delta_typed!(u32, U32),
            ValueType::U64 => delta_typed!(u64, U64),
            ValueType::F32 => delta_float!(f32, F32),
            ValueType::F64 => delta_float!(f64, F64),
            ValueType::ByteArray => false,
        }
    }
}

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScanCondition {
    Exact,
    GreaterThan,
    LessThan,
    Between,
    Unknown,
    Changed,
    Unchanged,
    Increased,
    Decreased,
    IncreasedBy,
    DecreasedBy,
}


#[cfg(test)]
mod byte_pattern_tests {
    use super::*;

    #[test]
    fn test_byte_pattern_parse() {
        let p = BytePattern::parse("48 8B ?? 90").unwrap();
        assert_eq!(p.len(), 4);
        assert_eq!(p.0[0], Some(0x48));
        assert_eq!(p.0[1], Some(0x8B));
        assert_eq!(p.0[2], None);
        assert_eq!(p.0[3], Some(0x90));
    }

    #[test]
    fn test_byte_pattern_matches() {
        let p = BytePattern::parse("48 8B ?? 90").unwrap();
        assert!(p.matches(&[0x48, 0x8B, 0xFF, 0x90]));
        assert!(p.matches(&[0x48, 0x8B, 0x00, 0x90]));
        assert!(!p.matches(&[0x48, 0x8B, 0xFF, 0x91]));
        assert!(!p.matches(&[0x48, 0x8C, 0xFF, 0x90]));
    }

    #[test]
    fn test_byte_pattern_too_short() {
        let p = BytePattern::parse("48 8B ?? 90").unwrap();
        assert!(!p.matches(&[0x48, 0x8B, 0xFF]));
    }

    #[test]
    fn test_byte_pattern_single_wildcard_char() {
        let p = BytePattern::parse("AA ? BB").unwrap();
        assert_eq!(p.len(), 3);
        assert!(p.matches(&[0xAA, 0x00, 0xBB]));
    }

    #[test]
    fn test_byte_pattern_empty() {
        let result = BytePattern::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_byte_pattern_invalid_hex() {
        let result = BytePattern::parse("48 ZZ 90");
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_value_pattern_matches() {
        let pattern = BytePattern::parse("48 8B ?? ?? 90").unwrap();
        let value = ScanValue::Pattern(pattern);
        let data = &[0x48, 0x8B, 0xAA, 0xBB, 0x90];
        assert!(value.matches(data, ScanCondition::Exact, None));
    }

    #[test]
    fn test_scan_value_pattern_no_match() {
        let pattern = BytePattern::parse("48 8B ?? ?? 90").unwrap();
        let value = ScanValue::Pattern(pattern);
        let data = &[0x48, 0x8B, 0xAA, 0xBB, 0x91];
        assert!(!value.matches(data, ScanCondition::Exact, None));
    }

    #[test]
    fn test_scan_value_pattern_other_conditions_false() {
        let pattern = BytePattern::parse("48 8B").unwrap();
        let value = ScanValue::Pattern(pattern);
        let data = &[0x48, 0x8B];
        // Non-exact conditions return false for patterns (via ByteArray path)
        assert!(!value.matches(data, ScanCondition::GreaterThan, None));
        assert!(!value.matches(data, ScanCondition::LessThan, None));
    }
}