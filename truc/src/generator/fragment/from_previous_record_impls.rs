use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::generator::{CAP, CAP_GENERIC};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FromKind {
    FromFull,
    FromUninit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IntoKind {
    IntoSimple,
    IntoAndOut,
}

pub struct FromPreviousRecordImplsGenerator;

impl FromPreviousRecordImplsGenerator {
    fn generate_from_previous_record_impl(
        record_spec: &RecordSpec,
        prev_record_spec: &RecordSpec,
        from_kind: FromKind,
        into_kind: IntoKind,
        scope: &mut Scope,
    ) {
        let from_type = format!(
            "({}<{}>, {})",
            prev_record_spec.capped_record_name,
            CAP,
            match from_kind {
                FromKind::FromFull => &record_spec.unpacked_record_in_name,
                FromKind::FromUninit => &record_spec.unpacked_uninit_record_in_name,
            }
        );
        let from_impl = scope
            .new_impl(match into_kind {
                IntoKind::IntoSimple => &record_spec.capped_record_name,
                IntoKind::IntoAndOut => &record_spec.record_and_unpacked_out_name,
            })
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait(format!("From<{}>", from_type));

        let plus_has_data = !record_spec.plus_data.is_empty();
        let uninit_plus_has_data = if from_kind == FromKind::FromUninit {
            record_spec
                .plus_data
                .iter()
                .any(|datum| !datum.allow_uninit())
        } else {
            false
        };

        let from_fn = from_impl
            .new_fn("from")
            .arg(
                if from_kind == FromKind::FromUninit || plus_has_data {
                    "(from, plus)"
                } else {
                    "(from, _plus)"
                },
                from_type,
            )
            .ret("Self");

        for datum in &record_spec.minus_data {
            from_fn.line(format!(
                "let {}{}: {} = unsafe {{ from.data.read({}) }};",
                match into_kind {
                    IntoKind::IntoSimple => "_",
                    IntoKind::IntoAndOut => "",
                },
                datum.name(),
                datum.type_name(),
                datum.offset(),
            ));
        }

        if from_kind == FromKind::FromUninit {
            from_fn.line(format!(
                "let {} = {}{}::from(plus);",
                if uninit_plus_has_data {
                    "plus"
                } else {
                    "_plus"
                },
                record_spec.unpacked_uninit_safe_record_in_name,
                record_spec
                    .plus_uninit_safe_generic
                    .as_ref()
                    .map_or_else(String::new, |generic| format!("::<{}>", generic.typed))
            ));
        }
        from_fn.line("let manually_drop = std::mem::ManuallyDrop::new(from);");
        from_fn.line(format!(
            "let {}data = unsafe {{ std::ptr::read(&manually_drop.data) }};",
            if from_kind == FromKind::FromFull && plus_has_data
                || from_kind == FromKind::FromUninit && uninit_plus_has_data
            {
                "mut "
            } else {
                ""
            }
        ));

        for datum in record_spec
            .plus_data
            .iter()
            .filter(|datum| from_kind == FromKind::FromFull || !datum.allow_uninit())
        {
            from_fn.line(format!(
                "unsafe {{ data.write({}, plus.{}); }}",
                datum.offset(),
                datum.name(),
            ));
        }
        match into_kind {
            IntoKind::IntoSimple => {
                from_fn.line("Self { data }");
            }
            IntoKind::IntoAndOut => {
                from_fn.line(format!(
                    "let record = {} {{ data }};",
                    record_spec.capped_record_name
                ));
                from_fn.line(format!(
                    "{} {{ record{} }}",
                    record_spec.record_and_unpacked_out_name,
                    record_spec
                        .minus_data
                        .iter()
                        .flat_map(|datum| [", ", datum.name()])
                        .collect::<String>()
                ));
            }
        }
    }
}

impl FragmentGenerator for FromPreviousRecordImplsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let prev_record_spec = if let Some(prev_record_spec) = specs.prev_record.as_ref() {
            prev_record_spec
        } else {
            return;
        };
        for (from_kind, into_kind) in [
            (FromKind::FromFull, IntoKind::IntoSimple),
            (FromKind::FromUninit, IntoKind::IntoSimple),
            (FromKind::FromFull, IntoKind::IntoAndOut),
            (FromKind::FromUninit, IntoKind::IntoAndOut),
        ] {
            Self::generate_from_previous_record_impl(
                specs.record,
                prev_record_spec,
                from_kind,
                into_kind,
                scope,
            );
        }
    }
}
