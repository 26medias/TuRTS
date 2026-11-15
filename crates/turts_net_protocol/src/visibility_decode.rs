//! Limit decoder allocation before serde containers reserve or grow from untrusted lengths.
use super::{StateTransactionError, StateTransactionPayload};
use serde::de::{
    Deserialize, DeserializeSeed, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};

struct Budget {
    left: usize,
    exhausted: bool,
}
impl Budget {
    fn charge<E: serde::de::Error>(&mut self, amount: usize) -> Result<(), E> {
        match self.left.checked_sub(amount) {
            Some(left) => {
                self.left = left;
                Ok(())
            }
            None => {
                self.exhausted = true;
                Err(E::custom("state decode allocation limit"))
            }
        }
    }
    fn value<T, E: serde::de::Error>(&mut self) -> Result<(), E> {
        // Includes vector capacity growth and container/node overhead, conservatively.
        self.charge::<E>(
            std::mem::size_of::<T>()
                .saturating_mul(2)
                .saturating_add(16),
        )
    }
}
struct Decoder<'a, D> {
    inner: D,
    budget: &'a mut Budget,
}
struct Wrap<'a, V> {
    inner: V,
    budget: &'a mut Budget,
}
struct Seed<'a, S> {
    inner: S,
    budget: &'a mut Budget,
}
impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<'_, S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.inner.deserialize(Decoder {
            inner: deserializer,
            budget: self.budget,
        })
    }
}
impl<'de, V: Visitor<'de>> Visitor<'de> for Wrap<'_, V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.expecting(f)
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        self.inner.visit_bool(value)
    }
    fn visit_i8<E: serde::de::Error>(self, value: i8) -> Result<Self::Value, E> {
        self.inner.visit_i8(value)
    }
    fn visit_i16<E: serde::de::Error>(self, value: i16) -> Result<Self::Value, E> {
        self.inner.visit_i16(value)
    }
    fn visit_i32<E: serde::de::Error>(self, value: i32) -> Result<Self::Value, E> {
        self.inner.visit_i32(value)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        self.inner.visit_i64(value)
    }
    fn visit_i128<E: serde::de::Error>(self, value: i128) -> Result<Self::Value, E> {
        self.inner.visit_i128(value)
    }
    fn visit_u8<E: serde::de::Error>(self, value: u8) -> Result<Self::Value, E> {
        self.inner.visit_u8(value)
    }
    fn visit_u16<E: serde::de::Error>(self, value: u16) -> Result<Self::Value, E> {
        self.inner.visit_u16(value)
    }
    fn visit_u32<E: serde::de::Error>(self, value: u32) -> Result<Self::Value, E> {
        self.inner.visit_u32(value)
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        self.inner.visit_u64(value)
    }
    fn visit_u128<E: serde::de::Error>(self, value: u128) -> Result<Self::Value, E> {
        self.inner.visit_u128(value)
    }
    fn visit_f32<E: serde::de::Error>(self, value: f32) -> Result<Self::Value, E> {
        self.inner.visit_f32(value)
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        self.inner.visit_f64(value)
    }
    fn visit_char<E: serde::de::Error>(self, value: char) -> Result<Self::Value, E> {
        self.inner.visit_char(value)
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.budget.charge::<E>(value.len().saturating_mul(2))?;
        self.inner.visit_str(value)
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.budget.charge::<E>(value.len().saturating_mul(2))?;
        self.inner.visit_borrowed_str(value)
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.budget
            .charge::<E>(value.capacity().saturating_mul(2))?;
        self.inner.visit_string(value)
    }
    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.budget.charge::<E>(value.len().saturating_mul(2))?;
        self.inner.visit_bytes(value)
    }
    fn visit_borrowed_bytes<E: serde::de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        self.budget.charge::<E>(value.len().saturating_mul(2))?;
        self.inner.visit_borrowed_bytes(value)
    }
    fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        self.budget
            .charge::<E>(value.capacity().saturating_mul(2))?;
        self.inner.visit_byte_buf(value)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_none()
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_unit()
    }
    fn visit_some<D: Deserializer<'de>>(self, inner: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_some(Decoder {
            inner,
            budget: self.budget,
        })
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, inner: D) -> Result<Self::Value, D::Error> {
        self.budget.value::<V::Value, D::Error>()?;
        self.inner.visit_newtype_struct(Decoder {
            inner,
            budget: self.budget,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, inner: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_seq(Access {
            inner,
            budget: self.budget,
        })
    }
    fn visit_map<A: MapAccess<'de>>(self, inner: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_map(Access {
            inner,
            budget: self.budget,
        })
    }
    fn visit_enum<A: EnumAccess<'de>>(self, inner: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_enum(Access {
            inner,
            budget: self.budget,
        })
    }
}
struct Access<'a, A> {
    inner: A,
    budget: &'a mut Budget,
}
impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for Access<'_, A> {
    type Error = A::Error;
    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        self.budget.value::<S::Value, Self::Error>()?;
        self.inner.next_element_seed(Seed {
            inner: seed,
            budget: self.budget,
        })
    }
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for Access<'_, A> {
    type Error = A::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        self.budget.charge::<Self::Error>(96)?;
        self.budget.value::<S::Value, Self::Error>()?;
        self.inner.next_key_seed(Seed {
            inner: seed,
            budget: self.budget,
        })
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.budget.value::<S::Value, Self::Error>()?;
        self.inner.next_value_seed(Seed {
            inner: seed,
            budget: self.budget,
        })
    }
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}
impl<'de, 'a, A: EnumAccess<'de>> EnumAccess<'de> for Access<'a, A> {
    type Error = A::Error;
    type Variant = Access<'a, A::Variant>;
    fn variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), Self::Error> {
        let (value, inner) = self.inner.variant_seed(Seed {
            inner: seed,
            budget: self.budget,
        })?;
        Ok((
            value,
            Access {
                inner,
                budget: self.budget,
            },
        ))
    }
}
impl<'de, A: VariantAccess<'de>> VariantAccess<'de> for Access<'_, A> {
    type Error = A::Error;
    fn unit_variant(self) -> Result<(), Self::Error> {
        self.inner.unit_variant()
    }
    fn newtype_variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.budget.value::<S::Value, Self::Error>()?;
        self.inner.newtype_variant_seed(Seed {
            inner: seed,
            budget: self.budget,
        })
    }
    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.tuple_variant(
            len,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.struct_variant(
            fields,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
}
impl<'de, D: Deserializer<'de>> Deserializer<'de> for Decoder<'_, D> {
    type Error = D::Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_any(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_bool(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_i8(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_i16(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_i32(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_i64(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_i128(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_u8(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_u16(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_u32(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_u64(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_u128(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_f32(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_f64(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_char(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_str(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_string(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_bytes(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_byte_buf(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_option(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_unit(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_seq(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_map(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_identifier(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_ignored_any(Wrap {
            inner: visitor,
            budget: self.budget,
        })
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_unit_struct(
            name,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_newtype_struct(
            name,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple(
            len,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_tuple_struct(
            name,
            len,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_struct(
            name,
            fields,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.deserialize_enum(
            name,
            variants,
            Wrap {
                inner: visitor,
                budget: self.budget,
            },
        )
    }
    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}
pub(super) fn payload(
    bytes: &[u8],
    limit: usize,
) -> Result<StateTransactionPayload, StateTransactionError> {
    let mut budget = Budget {
        left: limit,
        exhausted: false,
    };
    let mut postcard = postcard::Deserializer::from_bytes(bytes);
    let payload = StateTransactionPayload::deserialize(Decoder {
        inner: &mut postcard,
        budget: &mut budget,
    })
    .map_err(|error| {
        if matches!(error, postcard::Error::SerdeDeCustom) && budget.exhausted {
            StateTransactionError::MemoryLimit
        } else {
            StateTransactionError::MalformedPayload
        }
    })?;
    if !postcard
        .finalize()
        .map_err(|_| StateTransactionError::MalformedPayload)?
        .is_empty()
    {
        return Err(StateTransactionError::MalformedPayload);
    }
    Ok(payload)
}
