use crate::{
    chain::{Chain, ImportScope},
    dyn_node,
    graph::{DynNode, Graph, GraphBuilder, Node},
    stream::{NodeStream, NodeStreamSource},
    support::FullyQualifiedName,
};
use std::fmt::Write;

pub struct Dedup {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 1],
}

impl Node<1, 1> for Dedup {
    fn inputs(&self) -> &[NodeStream; 1] {
        &self.inputs
    }

    fn outputs(&self) -> &[NodeStream; 1] {
        &self.outputs
    }

    fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
        let thread = chain.get_thread_id_and_module_by_source(self.inputs[0].source(), &self.name);

        let local_name = self.name.last().expect("local name");
        let def =
            self.outputs[0].definition_fragments(&graph.chain_customizer().streams_module_name);
        let scope = chain.get_or_new_module_scope(
            self.name.iter().take(self.name.len() - 1),
            graph.chain_customizer(),
            thread.thread_id,
        );
        let mut import_scope = ImportScope::default();
        import_scope.add_import_with_error_type("streamink::stream::sync", "SyncStream");
        let node_fn = scope
            .new_fn(local_name)
            .vis("pub")
            .arg(
                "thread_control",
                format!("&mut thread_{}::ThreadControl", thread.thread_id),
            )
            .ret(def.impl_sync_stream);
        let input = thread.format_input(
            self.inputs[0].source(),
            graph.chain_customizer(),
            &mut import_scope,
        );
        let record_definition = &graph.record_definitions()[self.inputs[0].record_type()];
        let variant = record_definition
            .get_variant(self.inputs[0].variant_id())
            .unwrap_or_else(|| panic!("variant #{}", self.inputs[0].variant_id()));
        let mut eq = "|a, b| ".to_string();
        for (i, d) in variant.data().enumerate() {
            let datum = record_definition
                .get_datum_definition(d)
                .unwrap_or_else(|| panic!("datum #{}", d));
            if i > 0 {
                write!(eq, " &&\n        ").expect("write");
            }
            write!(eq, "a.{field}().eq(b.{field}())", field = datum.name()).expect("write");
        }
        crate::chain::fn_body(
            format!(
                r#"{input}
streamink::dedup::Dedup::new(
    input,
    {eq},
)"#,
                input = input,
                eq = eq,
            ),
            node_fn,
        );
        import_scope.import(scope, graph.chain_customizer());

        chain.update_thread_single_stream(thread.thread_id, &self.outputs[0]);
    }
}

dyn_node!(Dedup);

pub fn dedup(
    _graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
) -> Dedup {
    let [input] = inputs;
    let output = input.with_source(NodeStreamSource::from(name.clone()));
    Dedup {
        name,
        inputs: [input],
        outputs: [output],
    }
}
