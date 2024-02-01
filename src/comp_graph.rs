use std::collections::HashMap;

pub(crate) type CompNodeId = u32;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct CompNode {
    name: &'static str,
    id: CompNodeId,
    operands: Vec<CompNodeId>,
    post: Vec<CompNodeId>,
}

impl CompNode {
    fn new(
        id: CompNodeId,
        name: &'static str,
        operands: Vec<CompNodeId>,
        post: Vec<CompNodeId>,
    ) -> Self {
        Self {
            name,
            id,
            operands,
            post,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Comp {
    nodes: HashMap<CompNodeId, CompNode>,
    next_id: CompNodeId,
}

pub type CompError = String;

impl Comp {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: Default::default(),
        }
    }

    pub fn add_const(&mut self, name: &'static str) -> CompNodeId {
        self.add_node(name, vec![]).unwrap()
    }

    pub fn add_node(
        &mut self,
        name: &'static str,
        operands: Vec<CompNodeId>,
    ) -> Result<CompNodeId, CompError> {
        self.add_dependent(name, operands, vec![])
    }

    pub fn is_valid_id(&self, id: &CompNodeId) -> bool {
        (0..self.next_id).contains(id)
    }

    pub fn validate_ids<'a, I: Iterator<Item = &'a CompNodeId>>(
        &self,
        ids: I,
    ) -> Result<(), CompError> {
        for id in ids {
            if !self.is_valid_id(id) {
                return Err(format!("Invalid id: {}", id));
            }
        }

        Ok(())
    }

    pub fn add_dependent(
        &mut self,
        name: &'static str,
        operands: Vec<CompNodeId>,
        post: Vec<CompNodeId>,
    ) -> Result<CompNodeId, CompError> {
        self.validate_ids(operands.iter())?;
        self.validate_ids(post.iter())?;

        let id = self.next_id;
        self.next_id = id + 1;

        self.nodes
            .insert(id, CompNode::new(id, name, operands, post));

        Ok(id)
    }

    pub fn as_str(&self, id: &CompNodeId) -> Result<String, CompError> {
        if !self.is_valid_id(id) {
            return Err("Invalid id".to_owned());
        }

        let node = self.get(id).unwrap();
        let operands: Vec<_> = node
            .operands
            .iter()
            .map(|op| self.as_str(op))
            .collect::<Result<_, CompError>>()?;

        Ok(format!("{}({})", node.name, operands.join(", ")))
    }

    pub fn get(&self, id: &CompNodeId) -> Option<&CompNode> {
        self.nodes.get(id)
    }
}
