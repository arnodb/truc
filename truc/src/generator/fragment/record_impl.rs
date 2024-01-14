use std::ops::Deref;

use codegen::{Impl, Scope};
use itertools::Itertools;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::{
    generator::{CAP, CAP_GENERIC},
    record::definition::DatumDefinition,
};

#[derive(Debug)]
enum UninitKind<'a> {
    Full,
    Uninit { safe_record_name: &'a str },
}

pub struct RecordImplGenerator;

impl RecordImplGenerator {
    fn generate_constructor(
        record_spec: &RecordSpec,
        unpacked_record_name: &str,
        uninit_kind: UninitKind,
        record_impl: &mut Impl,
    ) {
        let has_data = !record_spec.data.is_empty();
        let new_fn = record_impl
            .new_fn(match uninit_kind {
                UninitKind::Full => "new",
                UninitKind::Uninit { .. } => "new_uninit",
            })
            .vis("pub")
            .arg(
                match (&uninit_kind, has_data) {
                    (UninitKind::Full, false) => "_from",
                    (UninitKind::Full, true) | (UninitKind::Uninit { .. }, _) => "from",
                },
                unpacked_record_name,
            )
            .ret("Self");
        let uninit_has_data = if let UninitKind::Uninit { safe_record_name } = uninit_kind {
            let uninit_has_data = record_spec.data.iter().any(|datum| !datum.allow_uninit());
            new_fn.line(format!(
                "let {} = {}{}::from(from);",
                if uninit_has_data { "from" } else { "_from" },
                safe_record_name,
                record_spec
                    .unpacked_uninit_safe_generic
                    .as_ref()
                    .map_or_else(String::new, |generic| format!("::<{}>", generic.typed))
            ));
            uninit_has_data
        } else {
            false
        };
        new_fn.line(format!(
            "let {}data = RecordMaybeUninit::new();",
            match (&uninit_kind, has_data, uninit_has_data) {
                (UninitKind::Full, true, _) | (UninitKind::Uninit { .. }, _, true) => "mut ",
                _ => "",
            },
        ));
        for datum in record_spec
            .data
            .iter()
            .filter(|datum| matches!(uninit_kind, UninitKind::Full) || !datum.allow_uninit())
        {
            new_fn.line(format!(
                "unsafe {{ data.write({}, from.{}); }}",
                datum.offset(),
                datum.name()
            ));
        }
        new_fn.line("Self { data }");
    }

    fn generate_unpacker(
        data: &[&DatumDefinition],
        unpacked_record_name: &str,
        record_impl: &mut Impl,
    ) {
        let unpack_fn = record_impl
            .new_fn("unpack")
            .arg_self()
            .vis("pub")
            .ret(unpacked_record_name);
        for datum in data {
            unpack_fn.line(format!(
                "let {}: {} = unsafe {{ self.data.read({}) }};",
                datum.name(),
                datum.type_name(),
                datum.offset(),
            ));
        }
        unpack_fn.line("std::mem::forget(self);");
        unpack_fn.line(format!(
            "{} {{ {} }}",
            unpacked_record_name,
            data.iter()
                .map(Deref::deref)
                .map(DatumDefinition::name)
                .join(", ")
        ));
    }
}

impl FragmentGenerator for RecordImplGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let record_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP);

        Self::generate_constructor(
            record_spec,
            &record_spec.unpacked_record_name,
            UninitKind::Full,
            record_impl,
        );

        Self::generate_constructor(
            record_spec,
            &record_spec.unpacked_uninit_record_name,
            UninitKind::Uninit {
                safe_record_name: &record_spec.unpacked_uninit_safe_record_name,
            },
            record_impl,
        );

        Self::generate_unpacker(
            &record_spec.data,
            &record_spec.unpacked_record_name,
            record_impl,
        );

        for datum in &record_spec.data {
            record_impl
                .new_fn(datum.name())
                .vis("pub")
                .arg_ref_self()
                .ret(format!("&{}", datum.type_name()))
                .line(format!(
                    "unsafe {{ self.data.get::<{}>({}) }}",
                    datum.type_name(),
                    datum.offset()
                ));

            record_impl
                .new_fn(&format!("{}_mut", datum.name()))
                .vis("pub")
                .arg_mut_self()
                .ret(format!("&mut {}", datum.type_name()))
                .line(format!(
                    "unsafe {{ self.data.get_mut::<{}>({}) }}",
                    datum.type_name(),
                    datum.offset()
                ));
        }
    }
}
