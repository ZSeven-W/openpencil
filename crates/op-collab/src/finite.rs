use std::fmt;

use jian_ops_schema::{image_table::with_save_scope, image_thumbs::ImageThumbSnapshot};
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize, Serializer,
};

#[derive(Debug)]
pub(crate) struct NonFiniteNumber;

impl fmt::Display for NonFiniteNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("non-finite floating-point value")
    }
}

impl std::error::Error for NonFiniteNumber {}

impl serde::ser::Error for NonFiniteNumber {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self
    }
}

pub(crate) fn validate_finite<T: Serialize>(value: &T) -> Result<(), NonFiniteNumber> {
    with_save_scope(&ImageThumbSnapshot::default(), |_| {
        value.serialize(FiniteValidator)
    })
}

#[derive(Clone, Copy)]
struct FiniteValidator;

impl Serializer for FiniteValidator {
    type Ok = ();
    type Error = NonFiniteNumber;
    type SerializeSeq = FiniteCompound;
    type SerializeTuple = FiniteCompound;
    type SerializeTupleStruct = FiniteCompound;
    type SerializeTupleVariant = FiniteCompound;
    type SerializeMap = FiniteCompound;
    type SerializeStruct = FiniteCompound;
    type SerializeStructVariant = FiniteCompound;

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(NonFiniteNumber)
        }
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(NonFiniteNumber)
        }
    }

    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(FiniteCompound)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(FiniteCompound)
    }
}

struct FiniteCompound;

impl SerializeSeq for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTuple for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleStruct for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleVariant for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeMap for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(FiniteValidator)
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStruct for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStructVariant for FiniteCompound {
    type Ok = ();
    type Error = NonFiniteNumber;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(FiniteValidator)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
