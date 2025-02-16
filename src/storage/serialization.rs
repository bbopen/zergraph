//! Module for serialization and deserialization logic.

/// Serializes data into a binary format.
pub fn serialize<T: ?Sized>(data: &T) -> Vec<u8> {
    // TODO: Implement serialization logic
    Vec::new()
}

/// Deserializes data from a binary format into a type that implements Deserialize.
#[allow(unused)]
pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    // TODO: Implement deserialization logic
    Err("Not implemented".to_string())
} 