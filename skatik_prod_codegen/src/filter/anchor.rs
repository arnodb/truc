use crate::{
    chain::{Chain, ImportScope},
    dyn_node,
    graph::{DynNode, Graph, GraphBuilder, Node},
    stream::{NodeStream, NodeStreamSource},
    support::FullyQualifiedName,
};
use skatik_prod_data::AnchorId;
use truc::record::definition::DatumDefinitionOverride;

pub struct Anchorize {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 1],
    anchor_field: String,
}

impl Node<1, 1> for Anchorize {
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
        crate::chain::fn_body(
            format!(
                r#"let mut seq: usize = 0;
{input}
input
    .and_then_map(move |record| {{
        let anchor = seq;
        seq = anchor + 1;
        Ok({record}::<
            {{ {prefix}MAX_SIZE }},
        >::from((
            record,
            {unpacked_record_in} {{ {anchor_field}: skatik_prod_data::AnchorId::new(anchor) }},
        )))
    }})"#,
                input = input,
                prefix = def.prefix,
                record = def.record,
                unpacked_record_in = def.unpacked_record_in,
                anchor_field = self.anchor_field,
            ),
            node_fn,
        );
        import_scope.import(scope, graph.chain_customizer());

        chain.update_thread_single_stream(thread.thread_id, &self.outputs[0]);
    }
}

dyn_node!(Anchorize);

pub fn anchorize(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    anchor_field: &str,
) -> Anchorize {
    let [input] = inputs;

    let anchor_table_id = graph.new_anchor_table();

    let variant_id = {
        let mut stream = graph
            .get_stream(input.record_type())
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.record_type()))
            .borrow_mut();

        stream.add_datum_override::<AnchorId<0>, _>(
            anchor_field,
            DatumDefinitionOverride {
                type_name: Some(format!("skatik_prod_data::AnchorId<{}>", anchor_table_id)),
                size: None,
                allow_uninit: Some(true),
            },
        );
        stream.close_record_variant()
    };

    let record_type = input.record_type().clone();
    Anchorize {
        name: name.clone(),
        inputs: [input],
        outputs: [NodeStream::new(
            record_type,
            variant_id,
            NodeStreamSource::from(name),
        )],
        anchor_field: anchor_field.to_string(),
    }
}
