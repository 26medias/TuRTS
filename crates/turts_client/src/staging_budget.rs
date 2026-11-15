//! Allocation-free conservative preflight for decoded state plus its candidate and scratch tables.
//! Count four copies of nested storage. Container overhead covers validation sets and map nodes.
use serde::{Serialize, Serializer, ser};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limit;
impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("client state staging budget exceeded")
    }
}
impl std::error::Error for Limit {}
impl ser::Error for Limit {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Self
    }
}

pub(crate) struct Budget {
    left: usize,
}
impl Budget {
    pub(crate) fn new(limit: usize) -> Self {
        Self { left: limit }
    }
    pub(crate) fn bytes(&mut self, bytes: usize) -> Result<(), Limit> {
        self.left = self.left.checked_sub(bytes).ok_or(Limit)?;
        Ok(())
    }
    pub(crate) fn value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.sized(value)
    }
    fn sized<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        let size = std::mem::size_of_val(value);
        self.bytes(
            size.checked_mul(4)
                .and_then(|n| n.checked_add(if size > 8 { 64 } else { 0 }))
                .ok_or(Limit)?,
        )?;
        value.serialize(self)
    }
}
macro_rules! scalar {
    ($($method:ident($ty:ty)),* $(,)?) => { $(fn $method(self, _: $ty) -> Result<(), Limit> { Ok(()) })* };
}
impl<'a> Serializer for &'a mut Budget {
    type Ok = ();
    type Error = Limit;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char)
    );
    fn serialize_str(self, value: &str) -> Result<(), Limit> {
        self.bytes(
            value
                .len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<(), Limit> {
        self.bytes(value.len().checked_mul(4).ok_or(Limit)?)
    }
    fn serialize_none(self) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Limit> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        value.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Limit> {
        Ok(self)
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<(), Limit> {
        struct Length(usize);
        impl fmt::Write for Length {
            fn write_str(&mut self, text: &str) -> fmt::Result {
                self.0 = self.0.checked_add(text.len()).ok_or(fmt::Error)?;
                Ok(())
            }
        }
        let mut length = Length(0);
        fmt::write(&mut length, format_args!("{value}")).map_err(|_| Limit)?;
        self.bytes(
            length
                .0
                .checked_mul(4)
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}
macro_rules! compound {
    ($trait:ident, $method:ident $(, $key:ident : $ty:ty)*) => {
        impl ser::$trait for &mut Budget {
            type Ok = (); type Error = Limit;
            fn $method<T: Serialize + ?Sized>(&mut self, $($key: $ty,)* value: &T) -> Result<(), Limit> {
                $(let _ = $key;)*
                self.sized(value)
            }
            fn end(self) -> Result<(), Limit> { Ok(()) }
        }
    };
}
compound!(SerializeSeq, serialize_element);
compound!(SerializeTuple, serialize_element);
compound!(SerializeTupleStruct, serialize_field);
compound!(SerializeTupleVariant, serialize_field);
// Struct fields live inside their containing row. Recurse without a second structural charge.
impl ser::SerializeStruct for &mut Budget {
    type Ok = ();
    type Error = Limit;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), Limit> {
        Ok(())
    }
}
impl ser::SerializeStructVariant for &mut Budget {
    type Ok = ();
    type Error = Limit;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Limit> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), Limit> {
        Ok(())
    }
}
impl ser::SerializeMap for &mut Budget {
    type Ok = ();
    type Error = Limit;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.sized(value)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.sized(value)
    }
    fn end(self) -> Result<(), Limit> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize)]
    struct Nested {
        rows: Vec<Vec<String>>,
    }
    #[test]
    fn counts_nested_string_and_container_expansion_before_staging() {
        let rows = Nested {
            rows: vec![vec!["a".repeat(4096)]],
        };
        assert_eq!(Budget::new(4096).value(&rows), Err(Limit));
        assert!(Budget::new(64 * 1024).value(&rows).is_ok());
    }
    #[test]
    fn large_valid_coverage_masks_fit_without_per_byte_node_overhead() {
        let masks = (vec![255u8; 131_072], vec![255u8; 131_072]);
        assert!(Budget::new(2 * 1024 * 1024).value(&masks).is_ok());
    }
    #[test]
    fn arithmetic_overflow_cannot_reopen_a_spent_budget() {
        let mut budget = Budget::new(10);
        assert_eq!(budget.bytes(usize::MAX), Err(Limit));
        assert_eq!(budget.bytes(11), Err(Limit));
    }
}
