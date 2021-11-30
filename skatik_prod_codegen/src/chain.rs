use crate::{
    stream::{NodeStream, NodeStreamSource},
    support::FullyQualifiedName,
};
use codegen::{Function, Module, Scope};
use itertools::Itertools;
use std::{collections::HashMap, ops::Deref};

struct ChainThread {
    id: usize,
    name: FullyQualifiedName,
    main: Option<FullyQualifiedName>,
    input_streams: Box<[NodeStream]>,
    output_streams: Box<[NodeStream]>,
    input_pipes: Option<Box<[usize]>>,
    output_pipes: Option<Box<[usize]>>,
}

#[derive(Clone)]
pub struct ChainSourceThread {
    pub thread_id: usize,
    pub stream_index: usize,
    pub pipe: Option<usize>,
}

impl ChainSourceThread {
    pub fn format_input(
        &self,
        source_name: &NodeStreamSource,
        customizer: &ChainCustomizer,
        import_scope: &mut ImportScope,
    ) -> String {
        if self.pipe.is_none() {
            format!(
                "let input = {chain_module}::{source_name}(thread_control);",
                chain_module = customizer.module_name,
                source_name = source_name
            )
        } else {
            import_scope.add_error_type();
            format!(
                r#"let input = {{
    let rx = thread_control.input_{stream_index}.take().expect("input {stream_index}");
    streamink::sync::mpsc::Receive::<_, {error_type}>::new(rx)
}};"#,
                stream_index = self.stream_index,
                error_type = customizer.error_type,
            )
        }
    }
}

#[derive(new)]
pub struct Chain<'a> {
    customizer: &'a ChainCustomizer,
    scope: &'a mut Scope,
    #[new(default)]
    threads: Vec<ChainThread>,
    #[new(default)]
    thread_by_source: HashMap<NodeStreamSource, ChainSourceThread>,
    #[new(default)]
    pipe_count: usize,
}

impl<'a> Chain<'a> {
    pub fn new_thread(
        &mut self,
        name: FullyQualifiedName,
        input_streams: Box<[NodeStream]>,
        output_streams: Box<[NodeStream]>,
        input_pipes: Option<Box<[usize]>>,
        supersede_output_sources: bool,
        main: Option<FullyQualifiedName>,
    ) -> usize {
        for output_stream in &*output_streams {
            if let Some(ChainSourceThread { thread_id, .. }) =
                self.thread_by_source.get(output_stream.source())
            {
                if !supersede_output_sources {
                    let thread = &self.threads[*thread_id];
                    panic!(
                        r#"Thread "{}" is already the source of "{}""#,
                        thread.name,
                        output_stream.source()
                    );
                }
            } else if supersede_output_sources {
                panic!(
                    r#"Cannot find the source of "{}" in order to supersede it"#,
                    output_stream.source()
                );
            }
        }
        let thread_id = self.threads.len();
        for (i, output_stream) in output_streams.iter().enumerate() {
            self.thread_by_source.insert(
                output_stream.source().clone(),
                ChainSourceThread {
                    thread_id,
                    stream_index: i,
                    pipe: None,
                },
            );
        }
        let output_pipes = if main.is_some() {
            Some(
                output_streams
                    .iter()
                    .map(|_| self.new_pipe())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        } else {
            None
        };
        self.threads.push(ChainThread {
            id: thread_id,
            name,
            main,
            input_streams,
            output_streams,
            input_pipes,
            output_pipes,
        });
        let name = format!("thread_{}", thread_id);
        let module = self.scope.new_module(&name).vis("pub").scope();
        for (path, ty) in &self.customizer.custom_module_imports {
            module.import(path, ty);
        }
        thread_id
    }

    pub fn update_thread_single_stream(&mut self, thread_id: usize, stream: &NodeStream) {
        let thread = self.threads.get_mut(thread_id).expect("thread");
        assert_eq!(thread.output_streams.len(), 1);
        self.thread_by_source
            .remove(thread.output_streams[0].source());
        thread.output_streams[0] = stream.clone();
        self.thread_by_source.insert(
            stream.source().clone(),
            ChainSourceThread {
                thread_id,
                stream_index: 0,
                pipe: None,
            },
        );
    }

    fn get_source_thread(&self, source: &NodeStreamSource) -> &ChainSourceThread {
        self.thread_by_source.get(source).unwrap_or_else(|| {
            panic!(
                r#"Thread for source "{}" not found, available sources are [{}]"#,
                source,
                self.thread_by_source.keys().join(", ")
            )
        })
    }

    pub fn get_thread_id_and_module_by_source(
        &mut self,
        source: &NodeStreamSource,
        new_thread_name: &FullyQualifiedName,
    ) -> ChainSourceThread {
        let source_thread = self.get_source_thread(source);
        let thread_id = source_thread.thread_id;
        let thread = &self.threads[thread_id];
        if let Some(output_pipes) = &thread.output_pipes {
            let input_pipe = output_pipes[source_thread.stream_index];
            let streams = Box::new([thread.output_streams[source_thread.stream_index].clone()]);
            let thread_id = self.new_thread(
                new_thread_name.clone(),
                streams.clone(),
                streams,
                Some(Box::new([input_pipe])),
                true,
                None,
            );
            ChainSourceThread {
                thread_id,
                stream_index: 0,
                pipe: Some(input_pipe),
            }
        } else {
            source_thread.clone()
        }
    }

    fn new_pipe(&mut self) -> usize {
        let pipe = self.pipe_count;
        self.pipe_count = pipe + 1;
        pipe
    }

    pub fn pipe_single_thread(&mut self, source: &NodeStreamSource) -> Option<usize> {
        let source_thread = self.get_source_thread(source).clone();
        let thread = &mut self.threads[source_thread.thread_id];
        if let Some(output_pipes) = &thread.output_pipes {
            return Some(output_pipes[source_thread.stream_index]);
        }
        let pipe = self.new_pipe();
        let thread = &mut self.threads[source_thread.thread_id];
        let name = format!("thread_{}", source_thread.thread_id);
        let scope = self
            .scope
            .get_module_mut(&name)
            .expect("thread module")
            .scope();
        let mut import_scope = ImportScope::default();
        if thread.output_pipes.is_none() {
            assert_eq!(thread.output_streams.len(), 1);
            import_scope.add_import_with_error_type("streamink::stream::sync", "SyncStream");
            let pipe_fn = scope
                .new_fn("skatik_pipe")
                .vis("pub")
                .arg("thread_control", "&mut ThreadControl")
                .ret(format!(
                    "impl FnOnce() -> Result<(), {error_type}>",
                    error_type = self.customizer.error_type_name(),
                ));
            let input = source_thread.format_input(source, self.customizer, &mut import_scope);
            fn_body(
                format!(
                    r#"let tx = thread_control.output_0.take().expect("output 0");
{input}
let mut input = input;
move || {{
    while let Some(record) = input.next()? {{
        tx.send(Some(record))?;
    }}
    tx.send(None)?;
    Ok(())
}}"#,
                    input = input,
                ),
                pipe_fn,
            );

            thread.output_pipes = Some(Box::new([pipe]));
            thread.main = Some(FullyQualifiedName::new(name).sub("skatik_pipe"));
        }
        import_scope.import(scope, self.customizer);
        Some(pipe)
    }

    pub fn set_thread_main(&mut self, thread_id: usize, main: FullyQualifiedName) {
        self.threads[thread_id].main = Some(main);
    }

    pub fn gen_chain(&mut self) {
        for thread in &self.threads {
            let name = format!("thread_{}", thread.id);
            let scope = self
                .scope
                .get_module_mut(&name)
                .expect("thread module")
                .scope();
            if thread.input_streams.len() > 0 {
                scope.import("std::sync::mpsc", "Receiver");
            }
            if thread.output_pipes.is_some() && thread.output_streams.len() > 0 {
                scope.import("std::sync::mpsc", "SyncSender");
            }
            let thread_struct = scope.new_struct("ThreadControl").vis("pub");
            for (i, input_stream) in thread.input_streams.iter().enumerate() {
                let def = input_stream.definition_fragments(&self.customizer.streams_module_name);
                thread_struct.field(
                    &format!("pub input_{}", i),
                    format!(
                        "Option<Receiver<Option<{record}<{{ {prefix}MAX_SIZE }}>>>>",
                        prefix = def.prefix,
                        record = def.record,
                    ),
                );
            }
            if thread.output_pipes.is_some() {
                for (i, output_stream) in thread.output_streams.iter().enumerate() {
                    let def =
                        output_stream.definition_fragments(&self.customizer.streams_module_name);
                    thread_struct.field(
                        &format!("pub output_{}", i),
                        format!(
                            "Option<SyncSender<Option<{record}<{{ {prefix}MAX_SIZE }}>>>>",
                            prefix = def.prefix,
                            record = def.record,
                        ),
                    );
                }
            }
        }

        let main_fn = self.scope.new_fn("main").vis("pub").ret(format!(
            "Result<(), {error_type}>",
            error_type = self.customizer.error_type,
        ));
        for pipe in 0..self.pipe_count {
            main_fn.line(format!(
                "let (tx_{pipe}, rx_{pipe}) = std::sync::mpsc::sync_channel(42);",
                pipe = pipe
            ));
        }
        for thread in &self.threads {
            main_fn.line(format!(
                r#"let mut thread_control_{thread_id} = thread_{thread_id}::ThreadControl {{"#,
                thread_id = thread.id
            ));
            if let Some(input_pipes) = &thread.input_pipes {
                for (index, pipe) in input_pipes.iter().enumerate() {
                    main_fn.line(format!(
                        "    input_{index}: Some(rx_{pipe}),",
                        index = index,
                        pipe = pipe
                    ));
                }
            }
            if let Some(output_pipes) = &thread.output_pipes {
                for (index, pipe) in output_pipes.iter().enumerate() {
                    main_fn.line(format!(
                        "    output_{index}: Some(tx_{pipe}),",
                        index = index,
                        pipe = pipe
                    ));
                }
            }
            main_fn.line("};");
            main_fn.line(format!(
                "let join_{thread_id} = std::thread::spawn({thread_main}(&mut thread_control_{thread_id}));",
                thread_id = thread.id,
                thread_main = thread.main.as_ref().expect("main"),
            ));
        }
        for thread in &self.threads {
            main_fn.line(format!(
                "join_{thread_id}.join().unwrap()?;",
                thread_id = thread.id,
            ));
        }
        main_fn.line("Ok(())");
    }

    pub fn get_or_new_module_scope<'i>(
        &mut self,
        path: impl IntoIterator<Item = &'i Box<str>>,
        chain_customizer: &ChainCustomizer,
        thread_id: usize,
    ) -> &mut Scope {
        let mut iter = path.into_iter();
        let customize_module = |module: &mut Module| {
            for (path, ty) in &chain_customizer.custom_module_imports {
                module.import(path, ty);
            }
            let thread_module = format!("thread_{}", thread_id);
            module.scope().import("super", &thread_module).vis("pub");
        };
        if let Some(first) = iter.next() {
            let module = self.scope.get_or_new_module(first);
            (customize_module)(module);
            iter.fold(module, |m, n| {
                let module = m.get_or_new_module(n).vis("pub");
                (customize_module)(module);
                module
            })
            .scope()
        } else {
            self.scope
        }
    }
}

pub const DEFAULT_CHAIN_ROOT_MODULE_NAME: [&str; 2] = ["crate", "chain"];
pub const DEFAULT_CHAIN_STREAMS_MODULE_NAME: &str = "streams";
pub const DEFAULT_CHAIN_ERROR_TYPE: [&str; 2] = ["skatik_prod_data", "SkatikError"];

pub struct ChainCustomizer {
    pub streams_module_name: FullyQualifiedName,
    pub module_name: FullyQualifiedName,
    pub custom_module_imports: Vec<(String, String)>,
    pub error_type: FullyQualifiedName,
}

impl ChainCustomizer {
    pub fn error_type_path(&self) -> String {
        self.error_type
            .iter()
            .take(self.error_type.len() - 1)
            .map(Deref::deref)
            .collect()
    }

    pub fn error_type_name(&self) -> String {
        self.error_type
            .iter()
            .last()
            .expect("error_type last")
            .to_string()
    }
}

impl Default for ChainCustomizer {
    fn default() -> Self {
        Self {
            streams_module_name: FullyQualifiedName::new_n(
                DEFAULT_CHAIN_ROOT_MODULE_NAME
                    .iter()
                    .chain([DEFAULT_CHAIN_STREAMS_MODULE_NAME].iter()),
            ),
            module_name: FullyQualifiedName::new_n(DEFAULT_CHAIN_ROOT_MODULE_NAME.iter()),
            custom_module_imports: vec![],
            error_type: FullyQualifiedName::new_n(DEFAULT_CHAIN_ERROR_TYPE.iter()),
        }
    }
}

#[derive(Default)]
pub struct ImportScope {
    fixed: Vec<(String, String)>,
    import_error_type: bool,
    used: bool,
}

impl ImportScope {
    pub fn add_import(&mut self, path: &str, ty: &str) {
        self.fixed.push((path.to_string(), ty.to_string()));
    }

    pub fn add_import_with_error_type(&mut self, path: &str, ty: &str) {
        self.add_import(path, ty);
        self.add_error_type();
    }

    pub fn add_error_type(&mut self) {
        self.import_error_type = true;
    }

    pub fn import(mut self, scope: &mut Scope, customizer: &ChainCustomizer) {
        for (path, ty) in &self.fixed {
            scope.import(path, ty);
        }
        if self.import_error_type {
            scope.import(&customizer.error_type_path(), &customizer.error_type_name());
        }
        self.used = true;
    }
}

impl Drop for ImportScope {
    fn drop(&mut self) {
        assert!(self.used);
    }
}

pub fn fn_body<T: ToString>(body: T, the_fn: &mut Function) {
    for line in body.to_string().split('\n') {
        the_fn.line(line);
    }
}
