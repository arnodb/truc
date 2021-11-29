use crate::{
    chain::Chain,
    dyn_node,
    graph::{DynNode, Graph, GraphBuilder, Node},
    stream::{NodeStream, NodeStreamSource, StreamRecordType},
    support::FullyQualifiedName,
};
use itertools::Itertools;
use std::fmt::Write;
use truc::record::definition::DatumDefinitionOverride;

pub struct Group {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 1],
    rs_stream: NodeStream,
    fields: Vec<String>,
    rs_field: String,
}

impl Node<1, 1> for Group {
    fn inputs(&self) -> &[NodeStream; 1] {
        &self.inputs
    }

    fn outputs(&self) -> &[NodeStream; 1] {
        &self.outputs
    }

    fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
        let thread = chain.get_thread_id_and_module_by_source(self.inputs[0].source(), &self.name);

        let local_name = self.name.last().expect("local name");
        let def_input =
            self.inputs[0].definition_fragments(&graph.chain_customizer().streams_module_name);
        let def =
            self.outputs[0].definition_fragments(&graph.chain_customizer().streams_module_name);
        let def_rs = self
            .rs_stream
            .definition_fragments(&graph.chain_customizer().streams_module_name);
        let scope = chain.get_or_new_module_scope(
            self.name.iter().take(self.name.len() - 1),
            graph.chain_customizer(),
            thread.thread_id,
        );
        let node_fn = scope
            .new_fn(local_name)
            .vis("pub")
            .arg(
                "thread_control",
                format!("&mut thread_{}::ThreadControl", thread.thread_id),
            )
            .ret(def.impl_sync_stream);
        let input = thread.format_input(self.inputs[0].source(), graph.chain_customizer());
        let fields = self.fields.iter().join(", ");
        let record_definition = &graph.record_definitions()[self.outputs[0].record_type()];
        let variant = record_definition
            .get_variant(self.outputs[0].variant_id())
            .unwrap_or_else(|| panic!("variant #{}", self.outputs[0].variant_id()));
        let mut eq = "|group, rec| ".to_string();
        for (i, datum) in variant
            .data()
            .filter_map(|d| {
                let datum = record_definition
                    .get_datum_definition(d)
                    .unwrap_or_else(|| panic!("datum #{}", d));
                if !self.fields.iter().any(|f| f == datum.name()) && datum.name() != self.rs_field {
                    Some(datum)
                } else {
                    None
                }
            })
            .enumerate()
        {
            if i > 0 {
                write!(eq, " &&\n        ").expect("write");
            }
            write!(
                eq,
                "group.{field}().eq(&rec.{field}())",
                field = datum.name()
            )
            .expect("write");
        }
        crate::chain::fn_body(
            format!(
                r#"{input}
streamink::group::Group::new(
    input,
    |rec| {{
        let {record_and_unpacked_out} {{ mut record, {fields} }} = {record_and_unpacked_out}::from((rec, {unpacked_record_in} {{ {rs_field}: Vec::new() }}));
        let rs_record = {record_rs}::new({unpacked_record_rs} {{ {fields} }});
        record.{rs_field}_mut().push(rs_record);
        record
    }},
    {eq},
    |group, rec| {{
        let {unpacked_record_input}{{ {fields}, .. }} = rec.unpack();
        let rs_record = {record_rs}::new({unpacked_record_rs} {{ {fields} }});
        group.{rs_field}_mut().push(rs_record);
    }},
)"#,
                input = input,
                unpacked_record_input = def_input.unpacked_record,
                record_and_unpacked_out = def.record_and_unpacked_out,
                unpacked_record_in = def.unpacked_record_in,
                record_rs = def_rs.record,
                unpacked_record_rs = def_rs.unpacked_record,
                fields = fields,
                rs_field = self.rs_field,
                eq = eq,
            ),
            node_fn,
        );

        chain.update_thread_single_stream(thread.thread_id, &self.outputs[0]);
    }
}

dyn_node!(Group);

pub fn group(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    fields: &[&str],
    rs_field: &str,
) -> Group {
    let [input] = inputs;

    let rs_record_type = StreamRecordType::from(name.sub("rs"));
    graph.new_stream(rs_record_type.clone());

    let (variant_id, rs_stream) = {
        let mut stream = graph
            .get_stream(input.record_type())
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.record_type()))
            .borrow_mut();

        let mut rs_stream = graph
            .get_stream(&rs_record_type)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, rs_record_type))
            .borrow_mut();

        for &field in fields {
            let datum = stream
                .get_variant_datum_definition_by_name(input.variant_id(), field)
                .unwrap_or_else(|| panic!(r#"datum "{}""#, field));
            rs_stream.copy_datum(datum);
            let datum_id = datum.id();
            stream.remove_datum(datum_id);
        }
        let rs_variant_id = rs_stream.close_record_variant();

        let module_name = graph
            .chain_customizer()
            .streams_module_name
            .sub_n(&**rs_record_type);
        stream.add_datum_override::<Vec<()>, _>(
            rs_field,
            DatumDefinitionOverride {
                type_name: Some(format!(
                    "Vec<{module_name}::Record{rs_variant_id}<{{ {module_name}::MAX_SIZE }}>>",
                    module_name = module_name,
                    rs_variant_id = rs_variant_id,
                )),
                size: None,
                allow_uninit: None,
            },
        );
        (
            stream.close_record_variant(),
            NodeStream::new(rs_record_type, rs_variant_id, NodeStreamSource::default()),
        )
    };

    let record_type = input.record_type().clone();
    Group {
        name: name.clone(),
        inputs: [input],
        outputs: [NodeStream::new(
            record_type,
            variant_id,
            NodeStreamSource::from(name),
        )],
        rs_stream,
        fields: fields.iter().map(ToString::to_string).collect::<Vec<_>>(),
        rs_field: rs_field.to_string(),
    }
}
