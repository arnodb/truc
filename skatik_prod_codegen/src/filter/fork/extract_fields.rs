use crate::{
    chain::Chain,
    dyn_node,
    graph::{DynNode, Graph, GraphBuilder, Node},
    stream::{NodeStream, NodeStreamSource, StreamRecordType},
    support::FullyQualifiedName,
};
use itertools::Itertools;

pub struct ExtractFields {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 2],
}

impl Node<1, 2> for ExtractFields {
    fn inputs(&self) -> &[NodeStream; 1] {
        &self.inputs
    }

    fn outputs(&self) -> &[NodeStream; 2] {
        &self.outputs
    }

    fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
        let input_pipe = chain.pipe_single_thread(self.inputs[0].source());

        let local_name = self.name.last().expect("local name");
        let thread_id = chain.new_thread(
            self.name.clone(),
            self.inputs.to_vec().into_boxed_slice(),
            self.outputs.to_vec().into_boxed_slice(),
            if let Some(pipe) = input_pipe {
                Some(Box::new([pipe]))
            } else {
                None
            },
            false,
            Some(self.name.clone()),
        );
        let scope = chain.get_or_new_module_scope(
            self.name.iter().take(self.name.len() - 1),
            graph.chain_customizer(),
            thread_id,
        );
        let node_fn = scope
            .new_fn(local_name)
            .vis("pub")
            .arg(
                "thread_control",
                format!("&mut thread_{}::ThreadControl", thread_id),
            )
            .ret("impl FnOnce() -> Result<(), SkatikError>");
        crate::chain::fn_body(
            r#"let rx = thread_control.input_0.take().expect("input 0");
let tx_0 = thread_control.output_0.take().expect("output 0");
let tx_1 = thread_control.output_1.take().expect("output 1");
move || {
    while let Some(record) = rx.recv()? {"#,
            node_fn,
        );
        let record_definition = &graph.record_definitions()[self.outputs[1].record_type()];
        let variant = record_definition
            .get_variant(self.outputs[1].variant_id())
            .unwrap_or_else(|| panic!("variant #{}", self.outputs[1].variant_id()));
        for d in variant.data() {
            let datum = record_definition
                .get_datum_definition(d)
                .unwrap_or_else(|| panic!("datum #{}", d));
            node_fn.line(format!(
                "        let {name} = {deref}record.{name}(){clone};",
                name = datum.name(),
                deref = if datum.allow_uninit() { "*" } else { "" },
                clone = if datum.allow_uninit() { "" } else { ".clone()" },
            ));
        }
        let def_1 =
            self.outputs[1].definition_fragments(&graph.chain_customizer().streams_module_name);
        let fields = variant
            .data()
            .map(|d| {
                let datum = record_definition
                    .get_datum_definition(d)
                    .unwrap_or_else(|| panic!("datum #{}", d));
                datum.name()
            })
            .join(", ");
        crate::chain::fn_body(
            format!(
                r#"        let record_1 = {record_1}::<
            {{ {prefix_1}MAX_SIZE }},
        >::new(
            {unpacked_record_1} {{ {fields} }}
        );
        tx_0.send(Some(record))?;
        tx_1.send(Some(record_1))?;
    }}
    tx_0.send(None)?;
    tx_1.send(None)?;
    Ok(())
}}"#,
                prefix_1 = def_1.prefix,
                record_1 = def_1.record,
                unpacked_record_1 = def_1.unpacked_record,
                fields = fields,
            ),
            node_fn,
        );
    }
}

dyn_node!(ExtractFields);

pub fn extract_fields(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    fields: &[&str],
) -> ExtractFields {
    let [input] = inputs;

    let extracted_stream_name = name.sub("extracted");
    let extracted_record_type = StreamRecordType::from(extracted_stream_name.clone());
    graph.new_stream(extracted_record_type.clone());

    let extracted_variant_id = {
        let stream = graph
            .get_stream(input.record_type())
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.record_type()))
            .borrow_mut();

        let mut extracted_stream = graph
            .get_stream(&extracted_record_type)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, extracted_record_type))
            .borrow_mut();

        for field in fields {
            extracted_stream.copy_datum(
                stream
                    .get_variant_datum_definition_by_name(input.variant_id(), field)
                    .unwrap_or_else(|| panic!(r#"datum "{}""#, field)),
            );
        }
        extracted_stream.close_record_variant()
    };

    let output_0 = input.with_source(NodeStreamSource::from(name.clone()));
    let output_1 = NodeStream::new(
        extracted_record_type,
        extracted_variant_id,
        NodeStreamSource::from(extracted_stream_name),
    );
    ExtractFields {
        name,
        inputs: [input],
        outputs: [output_0, output_1],
    }
}
