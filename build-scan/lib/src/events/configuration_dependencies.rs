use error::ParseError;

use super::{
    BodyDecoder, BuildWideConfigurationDependenciesEvent, ConfigurationResolutionFinishedEvent,
    ConfigurationResolutionStartedEvent, DecodedEvent,
};

const FIXED_ID_PREFIX_LEN: usize = 9;
const MAX_SCANNED_STRING_LEN: usize = 256;

fn is_human_readable_char(ch: char) -> bool {
    matches!(ch, ' '..='~')
}

fn is_human_readable_string(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    if value.len() < 4 && value != ":" {
        return false;
    }

    if !value.chars().all(is_human_readable_char) {
        return false;
    }

    value
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '/' | '.' | '-' | '_' | ' '))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn try_read_inline_string(data: &[u8], start: usize) -> Option<(String, usize)> {
    let mut pos = start;
    let raw_len = kryo::read_zigzag_i64(data, &mut pos).ok()?;
    if raw_len <= 0 {
        return None;
    }

    let char_count = usize::try_from(raw_len).ok()?;
    if char_count > MAX_SCANNED_STRING_LEN {
        return None;
    }

    let mut value = String::with_capacity(char_count);
    for _ in 0..char_count {
        let code_point = varint::read_unsigned_varint(data, &mut pos).ok()? as u32;
        let ch = char::from_u32(code_point)?;
        if !is_human_readable_char(ch) {
            return None;
        }
        value.push(ch);
    }

    if !is_human_readable_string(&value) {
        return None;
    }

    Some((value, pos))
}

fn extract_human_readable_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        if let Some((value, next_pos)) = try_read_inline_string(data, pos) {
            push_unique(&mut strings, value);
            pos = next_pos;
        } else {
            pos += 1;
        }
    }

    strings
}

fn read_event_id(body: &[u8]) -> i64 {
    if body.len() < FIXED_ID_PREFIX_LEN {
        return 0;
    }

    i64::from_le_bytes([
        body[1], body[2], body[3], body[4], body[5], body[6], body[7], body[8],
    ])
}

fn extract_resolution_labels(body: &[u8]) -> Vec<String> {
    if body.len() <= FIXED_ID_PREFIX_LEN {
        return Vec::new();
    }

    extract_human_readable_strings(&body[FIXED_ID_PREFIX_LEN..])
}

fn looks_like_dependency_artifact_label(value: &str) -> bool {
    value.ends_with(".jar") || value.ends_with(".klib") || value.ends_with(".pom")
}

pub struct ConfigurationResolutionStartedDecoder;

impl BodyDecoder for ConfigurationResolutionStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        Ok(DecodedEvent::ConfigurationResolutionStarted(
            ConfigurationResolutionStartedEvent {
                id: read_event_id(body),
                labels: extract_resolution_labels(body),
            },
        ))
    }
}

pub struct ConfigurationResolutionFinishedDecoder;

impl BodyDecoder for ConfigurationResolutionFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        Ok(DecodedEvent::ConfigurationResolutionFinished(
            ConfigurationResolutionFinishedEvent {
                id: read_event_id(body),
                labels: extract_resolution_labels(body),
            },
        ))
    }
}

pub struct BuildWideConfigurationDependenciesDecoder;

impl BodyDecoder for BuildWideConfigurationDependenciesDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let root_node_id = kryo::read_positive_varint_i64(body, &mut pos).ok();
        let artifact_labels = extract_human_readable_strings(body)
            .into_iter()
            .filter(|value| looks_like_dependency_artifact_label(value))
            .collect();

        Ok(DecodedEvent::BuildWideConfigurationDependencies(
            BuildWideConfigurationDependenciesEvent {
                root_node_id,
                artifact_labels,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_labels_from_configuration_resolution_started_body() {
        let data = hex::decode("0058b1263018de7781183a6275696c642d6c6f676963000601ce757e78a079c9")
            .unwrap();

        let decoder = ConfigurationResolutionStartedDecoder;
        let decoded = decoder.decode(&data).unwrap();

        match decoded {
            DecodedEvent::ConfigurationResolutionStarted(event) => {
                assert_ne!(event.id, 0);
                assert_eq!(event.labels, vec![":build-logic"]);
            }
            other => panic!("expected ConfigurationResolutionStarted, got {other:?}"),
        }
    }

    #[test]
    fn extracts_labels_from_configuration_resolution_finished_body() {
        let data = hex::decode(
            "0852044f1ae519475c03143a6170703a6275696c64163a6c6973743a6275696c64203a7574696c69746965733a6275696c6400",
        )
        .unwrap();

        let decoder = ConfigurationResolutionFinishedDecoder;
        let decoded = decoder.decode(&data).unwrap();

        match decoded {
            DecodedEvent::ConfigurationResolutionFinished(event) => {
                assert_ne!(event.id, 0);
                assert_eq!(
                    event.labels,
                    vec![":app:build", ":list:build", ":utilities:build"]
                );
            }
            other => panic!("expected ConfigurationResolutionFinished, got {other:?}"),
        }
    }

    #[test]
    fn extracts_build_wide_artifact_labels() {
        let data = hex::decode(
            "de0b090428616e6e6f746174696f6e732d31332e302e6a617296016f72672e677261646c652e696e7465726e616c2e636f6d706f6e656e742e6c6f63616c2e6d6f64656c2e4f7061717565436f6d706f6e656e7441727469666163744964656e746966696572090430636f6d6d6f6e732d6c616e67332d332e32302e302e6a6172030904306b6f746c696e2d7374646c69622d322e332e32312e6a61720309043e6a756e69742d6a7570697465722d706172616d732d352e31322e312e6a6172030904306a756e69742d6a7570697465722d352e31322e312e6a6172030904286f70656e74657374346a2d312e332e302e6a6172030904086d61696e0309042e636f6d6d6f6e732d746578742d312e31352e302e6a617203090432617069677561726469616e2d6170692d312e312e322e6a6172030904426a756e69742d706c6174666f726d2d636f6d6d6f6e732d312e31322e312e6a6172030904386a756e69742d6a7570697465722d6170692d352e31322e312e6a617203030018617274696661637454797065066a6172001930636c617373706174682d656e7472792d736e617073686f740019126469726563746f7279",
        )
        .unwrap();

        let decoder = BuildWideConfigurationDependenciesDecoder;
        let decoded = decoder.decode(&data).unwrap();

        match decoded {
            DecodedEvent::BuildWideConfigurationDependencies(event) => {
                assert_eq!(event.root_node_id, Some(1502));
                assert!(
                    event
                        .artifact_labels
                        .contains(&"annotations-13.0.jar".to_string())
                );
                assert!(
                    event
                        .artifact_labels
                        .contains(&"junit-jupiter-api-5.12.1.jar".to_string())
                );
                assert!(!event.artifact_labels.contains(&"artifactType".to_string()));
            }
            other => panic!("expected BuildWideConfigurationDependencies, got {other:?}"),
        }
    }
}
