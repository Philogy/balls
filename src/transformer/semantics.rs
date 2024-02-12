use crate::comp_graph::{CompNode, CompNodeId, Computation};
use crate::parser::Ident;
use std::collections::HashMap;

const RESERVED_EMPTY_IDENTIFIER: &str = "_";

#[derive(Clone, Debug, Default)]
pub struct SemanticContext {
    next_id: CompNodeId,
    pub nodes: Vec<(CompNode, Computation)>,
    ident_to_id: HashMap<Ident, CompNodeId>,
    last_write: HashMap<Ident, CompNodeId>,
    last_reads: HashMap<Ident, Vec<CompNodeId>>,
}

impl SemanticContext {
    pub fn new_id(&mut self) -> CompNodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn set_ident(&mut self, ident: Ident, id: CompNodeId) {
        if ident != RESERVED_EMPTY_IDENTIFIER {
            self.ident_to_id.insert(ident, id);
        }
    }

    pub fn get_ident(&self, ident: &Ident) -> Option<&CompNodeId> {
        self.ident_to_id.get(ident)
    }

    pub fn get_has_output(&self, id: CompNodeId) -> Result<bool, String> {
        match self.nodes.get(id) {
            Some((node, _)) => Ok(node.has_output),
            None => Err(format!("Invalid comp id {}", id)),
        }
    }

    pub fn get_last_write(&self, ident: &String) -> Option<CompNodeId> {
        let last = self.last_write.get(ident)?;
        Some(*last)
    }

    pub fn record_read(&mut self, dependency: &Ident, id: CompNodeId) -> Option<CompNodeId> {
        let reading = self.last_reads.entry(dependency.clone()).or_default();
        reading.push(id);

        let write_id = *self.last_write.get(dependency)?;

        Some(write_id)
    }

    pub fn record_write(&mut self, dependency: &Ident, id: CompNodeId) -> Vec<CompNodeId> {
        let mut prev_reads = self
            .last_reads
            .insert(dependency.clone(), Vec::new())
            .unwrap_or_default();

        prev_reads.extend(self.last_write.insert(dependency.clone(), id));

        prev_reads
    }
}
