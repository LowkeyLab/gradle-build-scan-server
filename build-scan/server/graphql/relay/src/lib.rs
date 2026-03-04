use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use juniper::ID;
use serde::{Deserialize, Serialize};

/// Default number of items per page when `first` is not specified
pub const DEFAULT_PAGE_SIZE: i32 = 10;
/// Maximum allowed page size to prevent excessive queries
pub const MAX_PAGE_SIZE: i32 = 100;
/// Minimum allowed page size
pub const MIN_PAGE_SIZE: i32 = 1;

/// Relay Global Object Identification ID
/// Format: base64("Type:identifier")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayId {
    /// The GraphQL type name (e.g., "BuildScan")
    pub type_name: String,
    /// The raw ID value
    pub raw_id: String,
}

impl RelayId {
    /// Encode a type name and ID into a Relay global ID
    pub fn encode(type_name: &str, id: &str) -> ID {
        let plain = format!("{}:{}", type_name, id);
        ID::from(BASE64.encode(plain.as_bytes()))
    }

    /// Decode a Relay global ID and extract components
    pub fn decode(encoded: &ID) -> Result<RelayId> {
        let encoded_str: &str = encoded;
        let bytes = BASE64
            .decode(encoded_str)
            .context("Invalid base64 encoding")?;

        let plain = String::from_utf8(bytes).context("Invalid UTF-8 in decoded ID")?;

        let parts: Vec<&str> = plain.splitn(2, ':').collect();

        match parts.as_slice() {
            [type_name, id] => Ok(RelayId {
                type_name: type_name.to_string(),
                raw_id: id.to_string(),
            }),
            _ => bail!("Invalid relay ID format: {}", plain),
        }
    }
}

/// Cursor for pagination in Relay connections
/// Contains a generic string value used for cursor-based pagination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// The value used for pagination
    pub value: String,
}

impl Cursor {
    /// Create a new cursor from a value
    pub fn new(value: String) -> Self {
        Self { value }
    }

    /// Encode cursor to base64 JSON string
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self)
            .expect("Cursor serialization should never fail with valid data");
        BASE64.encode(json.as_bytes())
    }

    /// Decode cursor from base64 JSON string
    pub fn decode(encoded: &str) -> Result<Self> {
        let bytes = BASE64
            .decode(encoded)
            .context("Invalid base64 encoding in cursor")?;

        let json_str = String::from_utf8(bytes).context("Invalid UTF-8 in cursor")?;

        let cursor: Cursor = serde_json::from_str(&json_str).context("Invalid JSON in cursor")?;

        Ok(cursor)
    }
}

/// Validate pagination parameters
///
/// Ensures the page size is within acceptable bounds.
/// Defaults to DEFAULT_PAGE_SIZE if None.
pub fn validate_pagination(first: Option<i32>) -> Result<i32> {
    let page_size = first.unwrap_or(DEFAULT_PAGE_SIZE);

    if page_size < MIN_PAGE_SIZE {
        bail!("Page size must be at least {}", MIN_PAGE_SIZE);
    }

    if page_size > MAX_PAGE_SIZE {
        return Ok(MAX_PAGE_SIZE);
    }

    Ok(page_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_relay_id() {
        let id = RelayId::encode("BuildScan", "123456");
        let id_str: &str = &id;
        assert!(!id_str.is_empty());
    }

    #[test]
    fn test_decode_relay_id_round_trip() {
        let encoded = RelayId::encode("BuildScan", "abc-123");
        let decoded = RelayId::decode(&encoded).unwrap();

        assert_eq!(decoded.type_name, "BuildScan");
        assert_eq!(decoded.raw_id, "abc-123");
    }

    #[test]
    fn test_decode_invalid_base64() {
        let invalid_id = ID::from("not-valid-base64!!!".to_string());
        let result = RelayId::decode(&invalid_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_malformed_id() {
        let plain = "NoColonHere";
        let encoded = ID::from(BASE64.encode(plain.as_bytes()));
        let result = RelayId::decode(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_cursor_encode() {
        let cursor = Cursor::new("123456".to_string());
        let encoded = cursor.encode();
        assert!(!encoded.is_empty());
        assert!(BASE64.decode(&encoded).is_ok());
    }

    #[test]
    fn test_cursor_round_trip() {
        let original = Cursor::new("cursor-value-123".to_string());
        let encoded = original.encode();
        let decoded = Cursor::decode(&encoded).unwrap();

        assert_eq!(decoded.value, "cursor-value-123");
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_cursor_decode_invalid_base64() {
        let result = Cursor::decode("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_cursor_decode_invalid_json() {
        let invalid_json = BASE64.encode(b"not valid json");
        let result = Cursor::decode(&invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pagination_default() {
        let result = validate_pagination(None).unwrap();
        assert_eq!(result, DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn test_validate_pagination_valid_value() {
        let result = validate_pagination(Some(25)).unwrap();
        assert_eq!(result, 25);
    }

    #[test]
    fn test_validate_pagination_too_small() {
        let result = validate_pagination(Some(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pagination_cap_at_max() {
        let result = validate_pagination(Some(200)).unwrap();
        assert_eq!(result, MAX_PAGE_SIZE);
    }

    #[test]
    fn test_validate_pagination_boundary_values() {
        // MIN_PAGE_SIZE should pass
        assert_eq!(
            validate_pagination(Some(MIN_PAGE_SIZE)).unwrap(),
            MIN_PAGE_SIZE
        );
        // MAX_PAGE_SIZE should pass
        assert_eq!(
            validate_pagination(Some(MAX_PAGE_SIZE)).unwrap(),
            MAX_PAGE_SIZE
        );
    }
}
