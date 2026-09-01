use std::cell::{Cell, RefCell};
use std::sync::Arc;

use crate::value::key::Key;
use serde::{Serialize, Serializer, ser};

use crate::value::utils::SerializationFailed;
use crate::value::{Map, SmartString, StringKind, Value, ValueInner};

/// Magic name we use in serde to mean we already have a Value
const VALUE_PASSTHROUGH_NAME: &str = "$__tera::private::Value";

thread_local! {
    /// True while a `try_from_serializable` call is running on this thread.
    static IN_VALUE_SERIALIZER: Cell<bool> = const { Cell::new(false) };
    /// We store the cloned Value in there
    static PASSTHROUGH_SLOT: RefCell<Option<Value>> = const { RefCell::new(None) };
}

/// Serializes into a Value, cloning any Value it encounters instead of re-serializing it
pub(crate) fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, SerializationFailed> {
    let prev = IN_VALUE_SERIALIZER.replace(true);
    let result = value.serialize(ValueSerializer);
    IN_VALUE_SERIALIZER.replace(prev);
    result
}

pub(crate) fn serialize_value<S: Serializer>(
    value: &Value,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if !IN_VALUE_SERIALIZER.get() {
        return value.serialize_inner(serializer);
    }
    PASSTHROUGH_SLOT.with(|slot| *slot.borrow_mut() = Some(value.clone()));
    let result = serializer.serialize_newtype_struct(VALUE_PASSTHROUGH_NAME, &ValueBody(value));
    PASSTHROUGH_SLOT.with(|slot| slot.borrow_mut().take());
    result
}

/// Custom struct we use with VALUE_PASSTHROUGH_NAME to serialize efficiently
struct ValueBody<'a>(&'a Value);

impl Serialize for ValueBody<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize_inner(serializer)
    }
}

pub struct ValueSerializer;

impl Serializer for ValueSerializer {
    type Ok = Value;
    type Error = SerializationFailed;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeStruct;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Bool(v).into())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::I64(v as i64).into())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::I64(v as i64).into())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::I64(v as i64).into())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::I64(v).into())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::I128(Box::new(v)).into())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::U64(v as u64).into())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::U64(v as u64).into())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::U64(v as u64).into())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::U64(v).into())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::U128(Box::new(v)).into())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::F64(v as f64).into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::F64(v).into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::String(SmartString::new(&v.to_string(), StringKind::Normal)).into())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::String(SmartString::new(v, StringKind::Normal)).into())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Bytes(Arc::new(v.to_vec())).into())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::None.into())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::None.into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::String(SmartString::new(variant, StringKind::Normal)).into())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        if name == VALUE_PASSTHROUGH_NAME
            && let Some(v) = PASSTHROUGH_SLOT.with(|slot| slot.borrow_mut().take())
        {
            return Ok(v);
        }
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let mut map = Map::with_capacity(1);
        map.insert(Key::Str(variant), value.serialize(self)?);
        Ok(ValueInner::Map(Arc::new(map)).into())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            name: variant,
            fields: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            entries: Map::with_capacity(len.unwrap_or(0)),
            key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeStruct {
            fields: Map::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            variant,
            fields: Map::with_capacity(len),
        })
    }
}

pub struct SerializeSeq {
    elements: Vec<Value>,
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Array(Arc::new(self.elements)).into())
    }
}

impl ser::SerializeTuple for SerializeSeq {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Array(Arc::new(self.elements)).into())
    }
}

impl ser::SerializeTupleStruct for SerializeSeq {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Array(Arc::new(self.elements)).into())
    }
}

pub struct SerializeTupleVariant {
    name: &'static str,
    fields: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.fields.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = Map::with_capacity(1);
        map.insert(
            Key::Str(self.name),
            ValueInner::Array(Arc::new(self.fields)).into(),
        );
        Ok(ValueInner::Map(Arc::new(map)).into())
    }
}

pub struct SerializeMap {
    entries: Map,
    key: Option<Key<'static>>,
}

struct MapKeySerializer;

impl Serializer for MapKeySerializer {
    type Ok = Key<'static>;
    type Error = SerializationFailed;

    type SerializeSeq = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeTuple = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeTupleStruct = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeTupleVariant = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeMap = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeStruct = ser::Impossible<Key<'static>, SerializationFailed>;
    type SerializeStructVariant = ser::Impossible<Key<'static>, SerializationFailed>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Key::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Key::I64(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(Key::I128(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Key::U64(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(Key::U128(v))
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        Ok(Key::String(Arc::from(s)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Key::String(Arc::from(v)))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Key::Str(variant))
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(SerializationFailed(
            "map key must be string, integer, or bool".to_string(),
        ))
    }
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key = key.serialize(MapKeySerializer)?;
        self.key = Some(key);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self.key.take().expect("missing key");
        let value = value.serialize(ValueSerializer)?;
        self.entries.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Map(Arc::new(self.entries)).into())
    }
}

pub struct SerializeStruct {
    fields: Map,
}

impl ser::SerializeStruct for SerializeStruct {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value = value.serialize(ValueSerializer)?;
        self.fields.insert(Key::Str(key), value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueInner::Map(Arc::new(self.fields)).into())
    }
}

pub struct SerializeStructVariant {
    variant: &'static str,
    fields: Map,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = SerializationFailed;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value = value.serialize(ValueSerializer)?;
        self.fields.insert(Key::Str(key), value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = Map::with_capacity(1);
        map.insert(
            Key::Str(self.variant),
            ValueInner::Map(Arc::new(self.fields)).into(),
        );
        Ok(ValueInner::Map(Arc::new(map)).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn get_map_value<'a>(value: &'a Value, key: Key<'a>) -> Option<&'a str> {
        value.as_map()?.get(&key)?.as_str()
    }

    #[test]
    fn bool_and_int_map_keys() {
        let value = Value::from_serializable(&HashMap::from([(true, "yes")]));
        assert_eq!(get_map_value(&value, Key::Bool(true)), Some("yes"));

        let value = Value::from_serializable(&HashMap::from([(-5i64, "neg")]));
        assert_eq!(get_map_value(&value, Key::I64(-5)), Some("neg"));

        let value = Value::from_serializable(&HashMap::from([(7u64, "pos")]));
        assert_eq!(get_map_value(&value, Key::U64(7)), Some("pos"));

        let value = Value::from_serializable(&HashMap::from([(i128::MIN, "min")]));
        assert_eq!(get_map_value(&value, Key::I128(i128::MIN)), Some("min"));

        let value = Value::from_serializable(&HashMap::from([(u128::MAX, "max")]));
        assert_eq!(get_map_value(&value, Key::U128(u128::MAX)), Some("max"));
    }

    #[test]
    fn float_map_key_fails() {
        let result = 3.15f64.serialize(MapKeySerializer);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("map key must be"));
    }

    #[test]
    fn from_serializable_value_is_cloned() {
        let value = Value::from_serializable(&HashMap::from([("a", "b")]));
        let round_tripped = Value::from_serializable(&value);
        assert!(std::ptr::eq(
            value.as_map().unwrap(),
            round_tripped.as_map().unwrap()
        ));
    }

    #[test]
    fn from_serializable_nested_values_are_cloned() {
        #[derive(serde::Serialize)]
        struct Section {
            name: &'static str,
            children: Vec<Value>,
        }
        let child = Value::from_serializable(&HashMap::from([("a", "b")]));
        let section = Value::from_serializable(&Section {
            name: "w",
            children: vec![child.clone()],
        });
        let map = section.as_map().unwrap();
        assert_eq!(map.get(&Key::Str("name")).unwrap().as_str(), Some("w"));
        let children = map.get(&Key::Str("children")).unwrap().as_array().unwrap();
        assert!(std::ptr::eq(
            child.as_map().unwrap(),
            children[0].as_map().unwrap()
        ));
    }

    #[test]
    fn regular_serialization_works() {
        struct KeyedByValue(Value);
        impl Serialize for KeyedByValue {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(1))?;
                ser::SerializeMap::serialize_entry(&mut map, &Value::from("k"), &self.0)?;
                ser::SerializeMap::end(map)
            }
        }
        let inner = Value::from_serializable(&HashMap::from([("a", "b")]));
        let value = Value::from_serializable(&KeyedByValue(inner.clone()));
        let entry = value.as_map().unwrap().get(&Key::Str("k")).unwrap();
        assert!(std::ptr::eq(
            inner.as_map().unwrap(),
            entry.as_map().unwrap()
        ));
    }
}
