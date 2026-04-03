use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A string type that deserializes from both JSON strings (`"123"`) and JSON
/// numbers (`123`). The Breitbandmessung API is inconsistent about whether
/// certain integer fields are returned as strings or numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InconsistentIntegerString(pub String);

impl fmt::Display for InconsistentIntegerString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for InconsistentIntegerString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for InconsistentIntegerString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(IisVisitor)
    }
}

struct IisVisitor;

impl Visitor<'_> for IisVisitor {
    type Value = InconsistentIntegerString;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string or integer")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(InconsistentIntegerString(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(InconsistentIntegerString(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(InconsistentIntegerString(v.to_string()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(InconsistentIntegerString(v.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_from_string() {
        let iis: InconsistentIntegerString = serde_json::from_str("\"123\"").unwrap();
        assert_eq!(iis.0, "123");
    }

    #[test]
    fn deserialize_from_integer() {
        let iis: InconsistentIntegerString = serde_json::from_str("456").unwrap();
        assert_eq!(iis.0, "456");
    }

    #[test]
    fn deserialize_from_negative_integer() {
        let iis: InconsistentIntegerString = serde_json::from_str("-1").unwrap();
        assert_eq!(iis.0, "-1");
    }

    #[test]
    fn serialize_to_string() {
        let iis = InconsistentIntegerString("42".into());
        let json = serde_json::to_string(&iis).unwrap();
        assert_eq!(json, "\"42\"");
    }
}
