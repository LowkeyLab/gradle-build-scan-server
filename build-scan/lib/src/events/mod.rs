use std::collections::HashMap;

use error::ParseError;

pub mod planned_node;
pub mod task_finished;
pub mod task_identity;
pub mod task_inputs_file_property;
pub mod task_inputs_file_property_root;
pub mod task_inputs_implementation;
pub mod task_inputs_property_names;
pub mod task_inputs_snapshotting_finished;
pub mod task_inputs_snapshotting_started;
pub mod task_inputs_value_properties;
pub mod task_started;
pub mod transform_execution_request;

pub trait BodyDecoder: Send + Sync {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError>;
}

#[derive(Debug, Clone)]
pub enum DecodedEvent {
    TaskIdentity(TaskIdentityEvent),
    TaskStarted(TaskStartedEvent),
    TaskFinished(TaskFinishedEvent),
    TaskInputsSnapshottingStarted(TaskInputsSnapshottingStartedEvent),
    TransformExecutionRequest(TransformExecutionRequestEvent),
    PlannedNode(PlannedNodeEvent),
    TaskInputsValueProperties(TaskInputsValuePropertiesEvent),
    TaskInputsPropertyNames(TaskInputsPropertyNamesEvent),
    TaskInputsImplementation(TaskInputsImplementationEvent),
    TaskInputsFileProperty(TaskInputsFilePropertyEvent),
    TaskInputsSnapshottingFinished(TaskInputsSnapshottingFinishedEvent),
    TaskInputsFilePropertyRoot(TaskInputsFilePropertyRootEvent),
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
pub struct TaskInputsSnapshottingStartedEvent {
    pub task: i64,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionRequestEvent {
    pub node_id: Option<i64>,
    pub identification_id: Option<i64>,
    pub execution_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PlannedNodeEvent {
    pub id: Option<i64>,
    pub dependencies: Vec<i64>,
    pub must_run_after: Vec<i64>,
    pub should_run_after: Vec<i64>,
    pub finalized_by: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsValuePropertiesEvent {
    pub id: Option<i64>,
    pub hashes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsPropertyNamesEvent {
    pub id: Option<i64>,
    pub value_inputs: Vec<String>,
    pub file_inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsImplementationEvent {
    pub id: Option<i64>,
    pub class_loader_hash: Option<Vec<u8>>,
    pub action_class_loader_hashes: Vec<Vec<u8>>,
    pub action_class_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyEvent {
    pub id: Option<i64>,
    pub attributes: Vec<String>,
    pub hash: Option<Vec<u8>>,
    pub roots: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingFinishedEvent {
    pub task: Option<i64>,
    pub result: Option<TaskInputsSnapshottingResult>,
    pub failure_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingResult {
    pub hash: Option<Vec<u8>>,
    pub implementation: Option<i64>,
    pub property_names: Option<i64>,
    pub value_inputs: Option<i64>,
    pub file_inputs: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyRootEvent {
    pub id: Option<i64>,
    pub file: FileRef,
    pub root_hash: Option<Vec<u8>>,
    pub children: Vec<FilePropertyRootChild>,
}

#[derive(Debug, Clone)]
pub struct FileRef {
    pub root: Option<u64>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FilePropertyRootChild {
    pub name: Option<String>,
    pub hash: Option<Vec<u8>>,
    pub parent: Option<i32>,
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
        registry.register(
            88,
            Box::new(task_inputs_file_property_root::TaskInputsFilePropertyRootDecoder),
        );
        registry.register(
            91,
            Box::new(task_inputs_implementation::TaskInputsImplementationDecoder),
        );
        registry.register(
            92,
            Box::new(task_inputs_property_names::TaskInputsPropertyNamesDecoder),
        );
        registry.register(
            94,
            Box::new(task_inputs_snapshotting_started::TaskInputsSnapshottingStartedDecoder),
        );
        registry.register(
            95,
            Box::new(task_inputs_value_properties::TaskInputsValuePropertiesDecoder),
        );
        registry.register(117, Box::new(task_identity::TaskIdentityDecoder));
        registry.register(119, Box::new(planned_node::PlannedNodeDecoder));
        registry.register(
            137,
            Box::new(transform_execution_request::TransformExecutionRequestDecoder),
        );
        registry.register(
            345,
            Box::new(task_inputs_file_property::TaskInputsFilePropertyDecoder),
        );
        registry.register(
            349,
            Box::new(task_inputs_snapshotting_finished::TaskInputsSnapshottingFinishedDecoder),
        );
        registry.register(1563, Box::new(task_started::TaskStartedDecoder));
        registry.register(2074, Box::new(task_finished::TaskFinishedDecoder));
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
