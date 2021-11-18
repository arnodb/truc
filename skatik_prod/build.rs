use skatik_prod_data::AnchorId;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap, HashSet},
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::Path,
};
use truc::{
    generator::generate,
    record::definition::{
        DatumDefinitionOverride, RecordDefinition, RecordDefinitionBuilder, RecordVariantId,
    },
};

struct StreamDefinition {
    record_definition_builder: RecordDefinitionBuilder,
}

impl Deref for StreamDefinition {
    type Target = RecordDefinitionBuilder;

    fn deref(&self) -> &Self::Target {
        &self.record_definition_builder
    }
}

impl DerefMut for StreamDefinition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.record_definition_builder
    }
}

#[derive(Clone)]
struct StreamAndVariant {
    name: String,
    variant_id: RecordVariantId,
}

trait Node {
    fn outputs(&self) -> &[StreamAndVariant];
}

struct NodeCluster<const OUT: usize> {
    outputs: [StreamAndVariant; OUT],
}

impl<const OUT: usize> Node for NodeCluster<OUT> {
    fn outputs(&self) -> &[StreamAndVariant] {
        &self.outputs
    }
}

#[derive(Default)]
struct GraphBuilder {
    record_definitions: HashMap<String, RefCell<RecordDefinitionBuilder>>,
    anchor_tables: HashSet<usize>,
}

impl GraphBuilder {
    fn new_stream(&mut self, name: String) {
        match self.record_definitions.entry(name) {
            Entry::Vacant(entry) => {
                let record_definition_builder = RecordDefinitionBuilder::new();
                entry.insert(record_definition_builder.into());
            }
            Entry::Occupied(entry) => {
                panic!(r#"Stream "{}" already exists"#, entry.key())
            }
        }
    }

    fn new_anchor_table(&mut self) -> usize {
        let anchor_table_id = self.anchor_tables.len();
        self.anchor_tables.insert(anchor_table_id);
        anchor_table_id
    }

    fn get_stream(&self, name: &str) -> Option<&RefCell<RecordDefinitionBuilder>> {
        self.record_definitions.get(name)
    }

    fn build(self) -> Graph {
        Graph {
            record_definitions: self
                .record_definitions
                .into_iter()
                .map(|(name, builder)| (name, builder.into_inner().build()))
                .collect(),
        }
    }
}

struct Graph {
    record_definitions: HashMap<String, RecordDefinition>,
}

impl Graph {
    fn generate(&self, output: &Path) -> Result<(), std::io::Error> {
        let mut file = File::create(&output.join("chain.rs")).unwrap();
        for (name, definition) in &self.record_definitions {
            write!(file, "pub mod {} {{\n\n", name)?;
            generate(definition, &mut file).unwrap();
            write!(&mut file, "\n\n}}")?;
        }
        Ok(())
    }
}

fn extract_fields(
    graph: &mut GraphBuilder,
    id: &str,
    inputs: [StreamAndVariant; 1],
    fields: &[&str],
) -> NodeCluster<2> {
    let [input] = inputs;

    let extracted_stream_name = format!("{}_extracted", id);
    graph.new_stream(extracted_stream_name.clone());

    let extracted_variant_id = {
        let stream = graph
            .get_stream(&input.name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.name))
            .borrow_mut();

        let mut extracted_stream = graph
            .get_stream(&extracted_stream_name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, extracted_stream_name))
            .borrow_mut();

        for field in fields {
            extracted_stream.copy_datum(
                stream
                    .get_variant_datum_definition_by_name(input.variant_id, field)
                    .unwrap_or_else(|| panic!(r#"datum "{}""#, field)),
            );
        }
        extracted_stream.close_record_variant()
    };

    NodeCluster {
        outputs: [
            input,
            StreamAndVariant {
                name: extracted_stream_name,
                variant_id: extracted_variant_id,
            },
        ],
    }
}

fn anchorize(
    graph: &mut GraphBuilder,
    _id: &str,
    inputs: [StreamAndVariant; 1],
    anchor_field: &str,
) -> NodeCluster<1> {
    let [input] = inputs;

    let anchor_table_id = graph.new_anchor_table();

    let variant_id = {
        let mut stream = graph
            .get_stream(&input.name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.name))
            .borrow_mut();

        stream.add_datum_override::<AnchorId<0>, _>(
            anchor_field,
            DatumDefinitionOverride {
                type_name: Some(format!("skatik_prod_data::AnchorId<{}>", anchor_table_id)),
                size: None,
                allow_uninit: None,
            },
        );
        stream.close_record_variant()
    };

    NodeCluster {
        outputs: [StreamAndVariant {
            name: input.name,
            variant_id,
        }],
    }
}

fn simplify_strings(
    _graph: &mut GraphBuilder,
    _id: &str,
    inputs: [StreamAndVariant; 1],
    _field: &str,
) -> NodeCluster<1> {
    NodeCluster { outputs: inputs }
}

fn reverse_strings(
    _graph: &mut GraphBuilder,
    _id: &str,
    inputs: [StreamAndVariant; 1],
    _field: &str,
) -> NodeCluster<1> {
    NodeCluster { outputs: inputs }
}

fn sort(
    _graph: &mut GraphBuilder,
    _id: &str,
    inputs: [StreamAndVariant; 1],
    _fields: &[&str],
) -> NodeCluster<1> {
    NodeCluster { outputs: inputs }
}

fn dedup(
    _graph: &mut GraphBuilder,
    _id: &str,
    inputs: [StreamAndVariant; 1],
    _field: &str,
) -> NodeCluster<1> {
    NodeCluster { outputs: inputs }
}

fn group(
    graph: &mut GraphBuilder,
    id: &str,
    inputs: [StreamAndVariant; 1],
    fields: &[&str],
    rs_field: &str,
) -> NodeCluster<1> {
    let [input] = inputs;

    let rs_stream_name = format!("{}_rs", id);
    graph.new_stream(rs_stream_name.clone());

    let variant_id = {
        let mut stream = graph
            .get_stream(&input.name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, input.name))
            .borrow_mut();

        let mut rs_stream = graph
            .get_stream(&rs_stream_name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, rs_stream_name))
            .borrow_mut();

        for &field in fields {
            let datum = stream
                .get_variant_datum_definition_by_name(input.variant_id, field)
                .unwrap_or_else(|| panic!(r#"datum "{}""#, field));
            rs_stream.copy_datum(datum);
            let datum_id = datum.id();
            stream.remove_datum(datum_id);
        }
        let rs_variant_id = rs_stream.close_record_variant();

        stream.add_datum_override::<Vec<()>, _>(
            rs_field,
            DatumDefinitionOverride {
                type_name: Some(format!(
                    "Vec<super::{}::Record{}<{{ super::{}::MAX_SIZE }}>>",
                    rs_stream_name, rs_variant_id, rs_stream_name
                )),
                size: None,
                allow_uninit: None,
            },
        );
        stream.close_record_variant()
    };

    NodeCluster {
        outputs: [StreamAndVariant {
            name: input.name,
            variant_id,
        }],
    }
}

fn build_rev_table(
    graph: &mut GraphBuilder,
    id: &str,
    inputs: [StreamAndVariant; 1],
    token_field: &str,
    reference_field: &str,
) -> NodeCluster<2> {
    let [input] = inputs;

    let extract_token = extract_fields(
        graph,
        &format!("{}_extract_token", id),
        [input],
        &[token_field, reference_field],
    );

    let reverse_token = reverse_strings(
        graph,
        &format!("{}_reverse_token", id),
        [extract_token.outputs()[1].clone()],
        token_field,
    );
    let sort_token = sort(
        graph,
        &format!("{}_sort_token", id),
        [reverse_token.outputs()[0].clone()],
        &[token_field],
    );

    NodeCluster {
        outputs: [
            extract_token.outputs()[0].clone(),
            sort_token.outputs()[0].clone(),
        ],
    }
}

fn build_sim_table(
    graph: &mut GraphBuilder,
    id: &str,
    inputs: [StreamAndVariant; 1],
    token_field: &str,
    reference_field: &str,
    ref_rs_field: &str,
) -> NodeCluster<2> {
    let [input] = inputs;

    let extract_token = extract_fields(
        graph,
        &format!("{}_extract_token", id),
        [input],
        &[token_field, reference_field],
    );

    let simplify_token = simplify_strings(
        graph,
        &format!("{}_simplify_token", id),
        [extract_token.outputs()[1].clone()],
        token_field,
    );
    let sort_token = sort(
        graph,
        &format!("{}_sort_token", id),
        [simplify_token.outputs()[0].clone()],
        &[token_field, reference_field],
    );
    let group = group(
        graph,
        &format!("{}_group", id),
        [sort_token.outputs()[0].clone()],
        &[reference_field],
        ref_rs_field,
    );

    NodeCluster {
        outputs: [
            extract_token.outputs()[0].clone(),
            group.outputs()[0].clone(),
        ],
    }
}

fn build_word_list(
    graph: &mut GraphBuilder,
    id: &str,
    inputs: [StreamAndVariant; 1],
    token_field: &str,
    anchor_field: &str,
    sim_anchor_field: &str,
    sim_rs_field: &str,
) -> NodeCluster<4> {
    let [input] = inputs;

    let sim = build_sim_table(
        graph,
        &format!("{}_sim", id),
        [input],
        token_field,
        anchor_field,
        sim_rs_field,
    );
    let rev = build_rev_table(
        graph,
        &format!("{}_rev", id),
        [sim.outputs()[0].clone()],
        token_field,
        anchor_field,
    );

    let anchorize = anchorize(
        graph,
        &format!("{}_anchorize", id),
        [sim.outputs()[1].clone()],
        sim_anchor_field,
    );
    let sim_rev = build_rev_table(
        graph,
        &format!("{}_sim_rev", id),
        [anchorize.outputs()[0].clone()],
        token_field,
        sim_anchor_field,
    );

    NodeCluster {
        outputs: [
            rev.outputs()[0].clone(),
            rev.outputs()[1].clone(),
            sim_rev.outputs()[0].clone(),
            sim_rev.outputs()[1].clone(),
        ],
    }
}

fn read(
    graph: &mut GraphBuilder,
    id: &str,
    _inputs: [StreamAndVariant; 0],
    _field: &str,
) -> NodeCluster<1> {
    let read_stream_name = format!("{}_read", id);
    graph.new_stream(read_stream_name.clone());

    let variant_id = {
        let mut stream = graph
            .get_stream(&read_stream_name)
            .unwrap_or_else(|| panic!(r#"stream "{}""#, read_stream_name))
            .borrow_mut();

        stream.add_datum::<Box<str>, _>("token");
        stream.close_record_variant()
    };

    NodeCluster {
        outputs: [StreamAndVariant {
            name: read_stream_name,
            variant_id,
        }],
    }
}

fn main() {
    let mut graph = GraphBuilder::default();

    let read_token = read(&mut graph, "read_token", [], "token");
    let sort_token = sort(
        &mut graph,
        "sort_token",
        [read_token.outputs()[0].clone()],
        &["token"],
    );
    let dedup_token = dedup(
        &mut graph,
        "dedup_token",
        [sort_token.outputs()[0].clone()],
        "token",
    );
    let anchorize = anchorize(
        &mut graph,
        "anchor",
        [dedup_token.outputs()[0].clone()],
        "anchor",
    );

    build_word_list(
        &mut graph,
        "word_list",
        [anchorize.outputs()[0].clone()],
        "token",
        "anchor",
        "sim_anchor",
        "sim_rs",
    );

    let graph = graph.build();

    let out_dir = std::env::var("OUT_DIR").unwrap();
    graph.generate(Path::new(&out_dir)).unwrap();
}
