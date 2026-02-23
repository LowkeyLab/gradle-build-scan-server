use std::collections::HashMap;

use error::ParseError;

pub mod task_finished;
pub mod task_identity;
pub mod task_started;

pub trait BodyDecoder: Send + Sync {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError>;
}

#[derive(Debug, Clone)]
pub enum DecodedEvent {
    TaskIdentity(TaskIdentityEvent),
    TaskStarted(TaskStartedEvent),
    TaskFinished(TaskFinishedEvent),
    Raw(RawEvent),
}

#[derive(Debug, Clone)]
pub struct TaskIdentityEvent {
    pub id: i64,
    pub build_path: String,
    pub task_path: String,
}

#[derive(Debug, Clone)]
pub struct TaskStartedEvent {
    pub id: i64,
    pub build_path: String,
    pub path: String,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskFinishedEvent {
    pub id: i64,
    pub path: String,
    pub outcome: Option<u64>,
    pub cacheable: Option<bool>,
    pub caching_disabled_reason_category: Option<String>,
    pub caching_disabled_explanation: Option<String>,
    pub origin_build_invocation_id: Option<String>,
    pub origin_build_cache_key: Option<Vec<u8>>,
    pub actionable: Option<bool>,
    pub skip_reason_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawEvent {
    pub wire_id: u16,
    pub body: Vec<u8>,
}

pub struct DecoderRegistry {
    decoders: HashMap<u16, Box<dyn BodyDecoder>>,
}

impl DecoderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            decoders: HashMap::new(),
        };
        registry.register(117, Box::new(task_identity::TaskIdentityDecoder));
        registry.register(1563, Box::new(task_started::TaskStartedDecoder));
        // task_finished will be registered in Task 9
        registry
    }

    pub fn register(&mut self, wire_id: u16, decoder: Box<dyn BodyDecoder>) {
        self.decoders.insert(wire_id, decoder);
    }

    pub fn decode(&self, wire_id: u16, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        match self.decoders.get(&wire_id) {
            Some(decoder) => decoder.decode(body),
            None => Ok(DecodedEvent::Raw(RawEvent {
                wire_id,
                body: body.to_vec(),
            })),
        }
    }
}
