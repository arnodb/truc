use crate::{
    chain::{Chain, ImportScope},
    dyn_node,
    graph::{DynNode, Graph, GraphBuilder, Node},
    stream::{NodeStream, NodeStreamSource},
    support::FullyQualifiedName,
};
use std::fmt::Write;

pub struct Sort {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 1],
    fields: Vec<String>,
}

impl Node<1, 1> for Sort {
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
        let mut cmp = "|a, b| ".to_string();
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 {
                write!(cmp, "\n         .then_with(|| ").expect("write");
            }
            write!(cmp, "a.{field}().cmp(b.{field}())", field = field).expect("write");
            if i > 0 {
                write!(cmp, ")").expect("write");
            }
        }
        crate::chain::fn_body(
            format!(
                r#"{input}
streamink::sort::SyncSort::new(
    input,
    {cmp},
)"#,
                input = input,
                cmp = cmp,
            ),
            node_fn,
        );
        import_scope.import(scope, graph.chain_customizer());

        chain.update_thread_single_stream(thread.thread_id, &self.outputs[0]);
    }
}

dyn_node!(Sort);

pub fn sort(
    _graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    fields: &[&str],
) -> Sort {
    let [input] = inputs;
    let output = input.with_source(NodeStreamSource::from(name.clone()));
    Sort {
        name,
        inputs: [input],
        outputs: [output],
        fields: fields.iter().map(ToString::to_string).collect::<Vec<_>>(),
    }
}
