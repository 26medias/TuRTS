//! Allocation-free conservative preflight for decoded state plus its candidate and scratch tables.
//! Count four copies of nested storage. Container overhead covers validation sets and map nodes.
//!
//! One row is charged one time. A struct or tuple header (or a direct value of more than 8 bytes)
//! is the charge for the inline storage of its row. The inline parts of that row (scalars, unit
//! variants, `Option` tags, nested structs and tuples) add nothing more. Heap parts (strings,
//! bytes, sequences, maps) are always charged, and each sequence or map element is a new row.
//! Before T-31 the inline parts were charged again: a terrain surface row of 28 bytes cost about
//! 830 bytes, so a 256 x 256 all-visible map used the full 64 MiB budget.
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
    /// The number of open rows whose inline storage is charged. Above 0, inline parts are free.
    inline: usize,
}
impl Budget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            left: limit,
            inline: 0,
        }
    }
    pub(crate) fn bytes(&mut self, bytes: usize) -> Result<(), Limit> {
        self.left = self.left.checked_sub(bytes).ok_or(Limit)?;
        Ok(())
    }
    /// Charge `rows` table rows of type `T` that already exist: four copies and 64 bytes of
    /// container overhead for each row. It needs no walk of the rows.
    pub(crate) fn rows<T>(&mut self, rows: usize) -> Result<(), Limit> {
        self.bytes(
            std::mem::size_of::<T>()
                .checked_mul(4)
                .and_then(|n| n.checked_add(64))
                .and_then(|n| n.checked_mul(rows))
                .ok_or(Limit)?,
        )
    }
    /// Charge storage that lives inline in its row: free when the row is charged.
    fn inline_bytes(&mut self, bytes: usize) -> Result<(), Limit> {
        if self.inline > 0 {
            Ok(())
        } else {
            self.bytes(bytes)
        }
    }
    /// Charge an inline header, then treat the parts up to the matching `close_row` as inline.
    fn open_row(&mut self, header: usize) -> Result<(), Limit> {
        self.inline_bytes(header)?;
        self.inline += 1;
        Ok(())
    }
    fn close_row(&mut self) {
        self.inline -= 1;
    }
    /// A sequence or map element is a new row: its inline storage is not in the parent row.
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        let parent = std::mem::replace(&mut self.inline, 0);
        let result = self.sized(value);
        self.inline = parent;
        result
    }
    pub(crate) fn value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.sized(value)
    }
    fn sized<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        let size = std::mem::size_of_val(value);
        // collect_seq passes references. Never treat a primitive reference as its storage width.
        // Large direct values add conservative inline storage; containers/rows count themselves too.
        if size <= 8 {
            return value.serialize(self);
        }
        self.open_row(
            size.checked_mul(4)
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )?;
        let result = value.serialize(&mut *self);
        self.close_row();
        result
    }
}
macro_rules! scalar {
    ($($method:ident($ty:ty)),* $(,)?) => { $(fn $method(self, _: $ty) -> Result<(), Limit> { self.inline_bytes(std::mem::size_of::<$ty>() * 4) })* };
}
impl Serializer for &mut Budget {
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
                .checked_add(std::mem::size_of::<String>())
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<(), Limit> {
        self.bytes(value.len().checked_mul(4).ok_or(Limit)?)
    }
    fn serialize_none(self) -> Result<(), Limit> {
        // Protocol nullable sequences contain ContentRef or SlotOutcome rows.
        self.inline_bytes(128)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Limit> {
        self.inline_bytes(32)?;
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Limit> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Limit> {
        self.inline_bytes(256)
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
        self.bytes(4 * std::mem::size_of::<Vec<()>>() + 64)?;
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self, Limit> {
        self.open_row(
            len.checked_mul(32)
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )?;
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Limit> {
        self.open_row(0)?;
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Limit> {
        self.open_row(0)?;
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self, Limit> {
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Self, Limit> {
        self.open_row(
            len.checked_mul(32)
                .and_then(|n| n.checked_add(64))
                .ok_or(Limit)?,
        )?;
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Limit> {
        self.open_row(0)?;
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
impl ser::SerializeSeq for &mut Budget {
    type Ok = ();
    type Error = Limit;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.element(value)
    }
    fn end(self) -> Result<(), Limit> {
        Ok(())
    }
}
// Tuple, tuple struct, struct and struct variant parts live inside their open row.
macro_rules! row {
    ($trait:ident, $method:ident $(, $key:ident : $ty:ty)*) => {
        impl ser::$trait for &mut Budget {
            type Ok = (); type Error = Limit;
            fn $method<T: Serialize + ?Sized>(&mut self, $($key: $ty,)* value: &T) -> Result<(), Limit> {
                $(let _ = $key;)*
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<(), Limit> { self.close_row(); Ok(()) }
        }
    };
}
row!(SerializeTuple, serialize_element);
row!(SerializeTupleStruct, serialize_field);
row!(SerializeTupleVariant, serialize_field);
row!(SerializeStruct, serialize_field, _name: &'static str);
row!(SerializeStructVariant, serialize_field, _name: &'static str);
impl ser::SerializeMap for &mut Budget {
    type Ok = ();
    type Error = Limit;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.bytes(64)?;
        self.element(value)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Limit> {
        self.element(value)
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
