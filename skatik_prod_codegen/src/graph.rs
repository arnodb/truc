use crate::{
    chain::Chain,
    stream::{NodeStream, StreamRecordType},
    support::FullyQualifiedName,
};
use codegen::Scope;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fs::File,
    path::Path,
};
use truc::record::definition::{RecordDefinition, RecordDefinitionBuilder};

pub trait Node<const IN: usize, const OUT: usize> {
    fn gen_chain(&self, graph: &Graph, chain: &mut Chain);
}

pub trait DynNode {
    fn dyn_gen_chain(&self, graph: &Graph, chain: &mut Chain);
}

#[macro_export]
macro_rules! dyn_node {
    ($t:ty) => {
        impl skatik_prod_codegen::graph::DynNode for $t {
            fn dyn_gen_chain(&self, graph: &Graph, chain: &mut Chain) {
                self.gen_chain(graph, chain)
            }
        }
    };
}

#[derive(new)]
pub struct NodeCluster<const IN: usize, const OUT: usize> {
    name: FullyQualifiedName,
    nodes: Vec<Box<dyn DynNode>>,
    inputs: [NodeStream; IN],
    outputs: [NodeStream; OUT],
}

impl<const IN: usize, const OUT: usize> NodeCluster<IN, OUT> {
    pub fn name(&self) -> &FullyQualifiedName {
        &self.name
    }

    pub fn inputs(&self) -> &[NodeStream; IN] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[NodeStream; OUT] {
        &self.outputs
    }
}

impl<const IN: usize, const OUT: usize> Node<IN, OUT> for NodeCluster<IN, OUT> {
    fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
        for node in &self.nodes {
            node.dyn_gen_chain(graph, chain);
        }
    }
}

impl<const IN: usize, const OUT: usize> DynNode for NodeCluster<IN, OUT> {
    fn dyn_gen_chain(&self, graph: &Graph, chain: &mut Chain) {
        self.gen_chain(graph, chain)
    }
}

#[derive(new)]
pub struct GraphBuilder {
    chain_streams_module_name: FullyQualifiedName,
    chain_module_name: FullyQualifiedName,
    #[new(default)]
    record_definitions: HashMap<StreamRecordType, RefCell<RecordDefinitionBuilder>>,
    #[new(default)]
    anchor_table_count: usize,
}

impl GraphBuilder {
    pub fn new_stream(&mut self, record_type: StreamRecordType) {
        match self.record_definitions.entry(record_type) {
            Entry::Vacant(entry) => {
                let record_definition_builder = RecordDefinitionBuilder::new();
                entry.insert(record_definition_builder.into());
            }
            Entry::Occupied(entry) => {
                panic!(r#"Stream "{}" already exists"#, entry.key())
            }
        }
    }

    pub fn new_anchor_table(&mut self) -> usize {
        let anchor_table_id = self.anchor_table_count;
        self.anchor_table_count = anchor_table_id + 1;
        anchor_table_id
    }

    pub fn get_stream(
        &self,
        record_type: &StreamRecordType,
    ) -> Option<&RefCell<RecordDefinitionBuilder>> {
        self.record_definitions.get(record_type)
    }

    pub fn build(self, entry_nodes: Vec<Box<dyn DynNode>>) -> Graph {
        Graph {
            chain_streams_module_name: self.chain_streams_module_name,
            chain_module_name: self.chain_module_name,
            record_definitions: self
                .record_definitions
                .into_iter()
                .map(|(name, builder)| (name, builder.into_inner().build()))
                .collect(),
            entry_nodes,
        }
    }

    pub fn chain_streams_module_name(&self) -> &FullyQualifiedName {
        &self.chain_streams_module_name
    }

    pub fn chain_module_name(&self) -> &FullyQualifiedName {
        &self.chain_module_name
    }
}

pub struct Graph {
    chain_streams_module_name: FullyQualifiedName,
    chain_module_name: FullyQualifiedName,
    record_definitions: HashMap<StreamRecordType, RecordDefinition>,
    entry_nodes: Vec<Box<dyn DynNode>>,
}

impl Graph {
    pub fn chain_streams_module_name(&self) -> &FullyQualifiedName {
        &self.chain_streams_module_name
    }

    pub fn chain_module_name(&self) -> &FullyQualifiedName {
        &self.chain_module_name
    }

    pub fn record_definitions(&self) -> &HashMap<StreamRecordType, RecordDefinition> {
        &self.record_definitions
    }

    pub fn generate(&self, output: &Path) -> Result<(), std::io::Error> {
        use std::io::Write;

        {
            let mut file = File::create(&output.join("chain_streams.rs")).unwrap();
            let mut scope = Scope::new();
            for (record_type, definition) in &self.record_definitions {
                let module = scope.get_or_new_module(&record_type[0]).vis("pub");
                let module = record_type
                    .iter()
                    .skip(1)
                    .fold(module, |m, n| m.get_or_new_module(n).vis("pub"))
                    .scope();
                module.raw(&truc::generator::generate(definition));
            }
            write!(file, "{}", scope.to_string()).unwrap();
        }
        {
            let mut scope = Scope::new();
            scope.import("crate::core", "*");
            scope.import("streamink::stream::sync", "SyncStream");

            let mut chain = Chain::new(
                self.chain_streams_module_name.clone(),
                self.chain_module_name.clone(),
                &mut scope,
            );

            for node in &self.entry_nodes {
                node.dyn_gen_chain(self, &mut chain);
            }

            chain.gen_chain();

            let mut file = File::create(&output.join("chain.rs")).unwrap();
            write!(file, "{}", scope.to_string()).unwrap();
        }
        Ok(())
    }
}
