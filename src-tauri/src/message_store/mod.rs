use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::parser::fhir::FhirResource;
use crate::parser::hl7::message::Hl7Message;

/// Thread-safe in-memory store for open messages.
pub struct MessageStore {
    messages: Arc<RwLock<HashMap<String, Hl7Message>>>,
    fhir_resources: Arc<RwLock<HashMap<String, FhirResource>>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
            fhir_resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a parsed HL7 message.
    pub fn insert(&self, id: String, message: Hl7Message) {
        let mut store = self.messages.write().unwrap();
        store.insert(id, message);
    }

    /// Get a clone of an HL7 message by ID.
    pub fn get(&self, id: &str) -> Option<Hl7Message> {
        let store = self.messages.read().unwrap();
        store.get(id).cloned()
    }

    /// Remove an HL7 message by ID.
    pub fn remove(&self, id: &str) -> Option<Hl7Message> {
        let mut store = self.messages.write().unwrap();
        store.remove(id)
    }

    /// Get the number of stored messages (HL7 + FHIR).
    pub fn len(&self) -> usize {
        let hl7 = self.messages.read().unwrap();
        let fhir = self.fhir_resources.read().unwrap();
        hl7.len() + fhir.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read a specific field's full content from the raw buffer.
    pub fn get_field_content(
        &self,
        message_id: &str,
        segment_idx: usize,
        field_idx: usize,
    ) -> Option<String> {
        let store = self.messages.read().unwrap();
        let msg = store.get(message_id)?;
        let segment = msg.segments.get(segment_idx)?;
        let field = segment.fields.iter().find(|f| f.position == field_idx)?;
        Some(field.span.as_str(&msg.raw).to_string())
    }

    // --- FHIR ---

    /// Store a parsed FHIR resource.
    pub fn insert_fhir(&self, id: String, resource: FhirResource) {
        let mut store = self.fhir_resources.write().unwrap();
        store.insert(id, resource);
    }

    /// Get a clone of a FHIR resource by ID.
    pub fn get_fhir(&self, id: &str) -> Option<FhirResource> {
        let store = self.fhir_resources.read().unwrap();
        store.get(id).cloned()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}
