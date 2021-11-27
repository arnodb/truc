use crate::{
    stream::{NodeStream, NodeStreamSource},
    support::FullyQualifiedName,
};
use codegen::{Function, Module, Scope};
use itertools::Itertools;
use std::collections::HashMap;

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
        chain_module: &FullyQualifiedName,
    ) -> String {
        if self.pipe.is_none() {
            format!(
                "let input = {chain_module}::{source_name}(thread_control);",
                chain_module = chain_module,
                source_name = source_name
            )
        } else {
            format!(
                r#"let input = {{
    let rx = thread_control.input_{stream_index}.take().expect("input {stream_index}");
    streamink::sync::mpsc::Receive::<_, SkatikError>::new(rx)
}};"#,
                stream_index = self.stream_index
            )
        }
    }
}

#[derive(new)]
pub struct Chain<'a> {
    streams_module_name: FullyQualifiedName,
    module_name: FullyQualifiedName,
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
        module.import("crate::core", "*");
        module.import("std::sync::mpsc", "{Receiver, SyncSender}");
        module.import("streamink::stream::sync", "SyncStream");
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
        if thread.output_pipes.is_none() {
            assert_eq!(thread.output_streams.len(), 1);
            let pipe_fn = scope
                .new_fn("skatik_pipe")
                .vis("pub")
                .arg("thread_control", "&mut ThreadControl")
                .ret("impl FnOnce() -> Result<(), SkatikError>");
            let input = source_thread.format_input(source, &self.module_name);
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
            let thread_struct = scope.new_struct("ThreadControl").vis("pub");
            for (i, input_stream) in thread.input_streams.iter().enumerate() {
                let def = input_stream.definition_fragments(&self.streams_module_name);
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
                    let def = output_stream.definition_fragments(&self.streams_module_name);
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

        let main_fn = self
            .scope
            .new_fn("main")
            .vis("pub")
            .ret("Result<(), SkatikError>");
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

    pub fn get_or_new_module_scope<'i, M>(
        &mut self,
        path: impl IntoIterator<Item = &'i Box<str>>,
        modify_module: M,
    ) -> &mut Scope
    where
        M: Fn(&mut Module),
    {
        let mut iter = path.into_iter();
        if let Some(first) = iter.next() {
            let module = self.scope.get_or_new_module(first);
            (modify_module)(module);
            iter.fold(module, |m, n| {
                let module = m.get_or_new_module(n).vis("pub");
                (modify_module)(module);
                module
            })
            .scope()
        } else {
            self.scope
        }
    }
}

pub fn fn_body<T: ToString>(body: T, the_fn: &mut Function) {
    for line in body.to_string().split('\n') {
        the_fn.line(line);
    }
}
