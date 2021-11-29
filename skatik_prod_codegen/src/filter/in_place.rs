use crate::{chain::Chain, graph::Graph, stream::NodeStream, support::FullyQualifiedName};
use codegen::Function;

struct InPlaceFilter {
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    outputs: [NodeStream; 1],
}

impl InPlaceFilter {
    fn gen_chain<F>(&self, graph: &Graph, chain: &mut Chain, f: F)
    where
        F: FnOnce(&mut Function),
    {
        let thread = chain.get_thread_id_and_module_by_source(self.inputs[0].source(), &self.name);

        let local_name = self.name.last().expect("local name");
        let def =
            self.outputs[0].definition_fragments(&graph.chain_customizer().streams_module_name);
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
        crate::chain::fn_body(
            format!(
                r#"{input}
input
    .and_then_map(|mut record| {{"#,
                input = input,
            ),
            node_fn,
        );
        f(node_fn);
        crate::chain::fn_body(
            r#"Ok(record)
    })"#,
            node_fn,
        );

        chain.update_thread_single_stream(thread.thread_id, &self.outputs[0]);
    }
}

pub mod string {
    use super::InPlaceFilter;
    use crate::dyn_node;
    use crate::graph::{DynNode, GraphBuilder};
    use crate::stream::NodeStreamSource;
    use crate::support::FullyQualifiedName;
    use crate::{
        chain::Chain,
        graph::{Graph, Node},
        stream::NodeStream,
    };

    pub struct ToLowercase {
        in_place: InPlaceFilter,
        fields: Box<[Box<str>]>,
        string_to_type: Option<Box<str>>,
    }

    impl Node<1, 1> for ToLowercase {
        fn inputs(&self) -> &[NodeStream; 1] {
            &self.in_place.inputs
        }

        fn outputs(&self) -> &[NodeStream; 1] {
            &self.in_place.outputs
        }

        fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
            self.in_place.gen_chain(graph, chain, |node_fn| {
                for field in &*self.fields {
                    crate::chain::fn_body(
                        format!(
                            concat!(
                                "*record.{field}_mut() = ",
                                "record.{field}()",
                                ".to_lowercase(){string_to_type};"
                            ),
                            field = field,
                            string_to_type = if let Some(stt) = &self.string_to_type {
                                stt
                            } else {
                                ""
                            },
                        ),
                        node_fn,
                    );
                }
            });
        }
    }

    dyn_node!(ToLowercase);

    pub fn to_lowercase<I, F>(
        _graph: &mut GraphBuilder,
        name: FullyQualifiedName,
        inputs: [NodeStream; 1],
        fields: I,
    ) -> ToLowercase
    where
        I: IntoIterator<Item = F>,
        F: Into<Box<str>>,
    {
        let [input] = inputs;
        let output = input.with_source(NodeStreamSource::from(name.clone()));
        ToLowercase {
            in_place: InPlaceFilter {
                name,
                inputs: [input],
                outputs: [output],
            },
            fields: fields
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            string_to_type: None,
        }
    }

    pub fn to_lowercase_boxed_str<I, F>(
        _graph: &mut GraphBuilder,
        name: FullyQualifiedName,
        inputs: [NodeStream; 1],
        fields: I,
    ) -> ToLowercase
    where
        I: IntoIterator<Item = F>,
        F: Into<Box<str>>,
    {
        let [input] = inputs;
        let output = input.with_source(NodeStreamSource::from(name.clone()));
        ToLowercase {
            in_place: InPlaceFilter {
                name,
                inputs: [input],
                outputs: [output],
            },
            fields: fields
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            string_to_type: Some(".into_boxed_str()".to_string().into_boxed_str()),
        }
    }

    pub struct ReverseChars {
        in_place: InPlaceFilter,
        fields: Box<[Box<str>]>,
        string_to_type: Option<Box<str>>,
    }

    impl Node<1, 1> for ReverseChars {
        fn inputs(&self) -> &[NodeStream; 1] {
            &self.in_place.inputs
        }

        fn outputs(&self) -> &[NodeStream; 1] {
            &self.in_place.outputs
        }

        fn gen_chain(&self, graph: &Graph, chain: &mut Chain) {
            self.in_place.gen_chain(graph, chain, |node_fn| {
                for field in &*self.fields {
                    crate::chain::fn_body(
                        format!(
                            concat!(
                                "*record.{field}_mut() = ",
                                "record.{field}()",
                                ".chars().rev().collect::<String>(){string_to_type};"
                            ),
                            field = field,
                            string_to_type = if let Some(stt) = &self.string_to_type {
                                stt
                            } else {
                                ""
                            },
                        ),
                        node_fn,
                    );
                }
            });
        }
    }

    dyn_node!(ReverseChars);

    pub fn reverse_chars<I, F>(
        _graph: &mut GraphBuilder,
        name: FullyQualifiedName,
        inputs: [NodeStream; 1],
        fields: I,
    ) -> ReverseChars
    where
        I: IntoIterator<Item = F>,
        F: Into<Box<str>>,
    {
        let [input] = inputs;
        let output = input.with_source(NodeStreamSource::from(name.clone()));
        ReverseChars {
            in_place: InPlaceFilter {
                name,
                inputs: [input],
                outputs: [output],
            },
            fields: fields
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            string_to_type: None,
        }
    }

    pub fn reverse_chars_boxed_str<I, F>(
        _graph: &mut GraphBuilder,
        name: FullyQualifiedName,
        inputs: [NodeStream; 1],
        fields: I,
    ) -> ReverseChars
    where
        I: IntoIterator<Item = F>,
        F: Into<Box<str>>,
    {
        let [input] = inputs;
        let output = input.with_source(NodeStreamSource::from(name.clone()));
        ReverseChars {
            in_place: InPlaceFilter {
                name,
                inputs: [input],
                outputs: [output],
            },
            fields: fields
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            string_to_type: Some(".into_boxed_str()".to_string().into_boxed_str()),
        }
    }
}
