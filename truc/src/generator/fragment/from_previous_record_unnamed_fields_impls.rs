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

pub struct FromPreviousRecordUnnamedFieldsImplsGenerator;

impl FromPreviousRecordUnnamedFieldsImplsGenerator {
    fn generate_from_previous_record_unnamed_fields_impl(
        record_spec: &RecordSpec,
        prev_record_spec: &RecordSpec,
        from_kind: FromKind,
        into_kind: IntoKind,
        scope: &mut Scope,
    ) {
        if from_kind == FromKind::FromUninit
            && !record_spec
                .plus_data
                .iter()
                .any(|datum| datum.details().allow_uninit())
        {
            return;
        }

        let (from_plus_args, from_plus_types) = match from_kind {
            FromKind::FromFull => {
                let iter = record_spec.plus_data.iter();
                (
                    iter.clone()
                        .map(|datum| datum.name())
                        .collect::<Vec<&str>>(),
                    iter.map(|datum| datum.details().type_name())
                        .collect::<Vec<&str>>(),
                )
            }
            FromKind::FromUninit => {
                let iter = record_spec
                    .plus_data
                    .iter()
                    .filter(|datum| !datum.details().allow_uninit());

                (
                    iter.clone()
                        .map(|datum| datum.name())
                        .collect::<Vec<&str>>(),
                    iter.map(|datum| datum.details().type_name())
                        .collect::<Vec<&str>>(),
                )
            }
        };

        let ty = format!(
            "({}<{}>, {})",
            prev_record_spec.capped_record_name,
            CAP,
            match from_plus_types.len() {
                0 => "()".to_owned(),
                1 => from_plus_types[0].to_owned(),
                _ => format!("({})", from_plus_types.join(", ")),
            }
        );

        let from_impl = scope
            .new_impl(match into_kind {
                IntoKind::IntoSimple => &record_spec.capped_record_name,
                IntoKind::IntoAndOut => &record_spec.record_and_unpacked_out_name,
            })
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait(format!("From<{}>", ty));

        let plus_has_data = !record_spec.plus_data.is_empty();
        let uninit_plus_has_data = if from_kind == FromKind::FromUninit {
            record_spec
                .plus_data
                .iter()
                .any(|datum| !datum.details().allow_uninit())
        } else {
            false
        };

        let from_fn = from_impl
            .new_fn("from")
            .arg(
                &format!(
                    "(from, {})",
                    match from_plus_args.len() {
                        0 => "()".to_owned(),
                        1 => from_plus_args[0].to_owned(),
                        _ => format!("({})", from_plus_args.join(", ")),
                    }
                ),
                ty,
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
                datum.details().type_name(),
                datum.details().offset(),
            ));
        }

        match from_kind {
            FromKind::FromFull => {
                if plus_has_data {
                    from_fn.line(format!(
                        "let plus = {} {{ {} }};",
                        record_spec.unpacked_record_in_name,
                        from_plus_args.join(", ")
                    ));
                }
            }
            FromKind::FromUninit => {
                if uninit_plus_has_data {
                    from_fn.line(format!(
                        "let plus = {}{}::from({} {{ { } }});",
                        record_spec.unpacked_uninit_safe_record_in_name,
                        record_spec
                            .plus_uninit_safe_generic
                            .as_ref()
                            .map_or_else(String::new, |generic| format!("::<{}>", generic.typed)),
                        record_spec.unpacked_uninit_record_in_name,
                        from_plus_args.join(", ")
                    ));
                }
            }
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
            .filter(|datum| from_kind == FromKind::FromFull || !datum.details().allow_uninit())
        {
            from_fn.line(format!(
                "unsafe {{ data.write({}, plus.{}); }}",
                datum.details().offset(),
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

impl FragmentGenerator for FromPreviousRecordUnnamedFieldsImplsGenerator {
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
            Self::generate_from_previous_record_unnamed_fields_impl(
                specs.record,
                prev_record_spec,
                from_kind,
                into_kind,
                scope,
            );
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::collections::BTreeSet;

    use maplit::btreeset;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        generator::{config::GeneratorConfig, generate_variant, tests::assert_fragment_eq},
        record::{
            definition::builder::native::NativeRecordDefinitionBuilder,
            type_resolver::HostTypeResolver,
        },
    };

    #[test]
    fn should_generate_empty_impls() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(FromPreviousRecordUnnamedFieldsImplsGenerator)
                as Box<dyn FragmentGenerator>]);

        let mut scope = Scope::new();
        let mut type_size_assertions = BTreeSet::new();

        generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().next().expect("variant"),
            None,
            &config,
            &mut scope,
            &mut type_size_assertions,
        );

        assert_fragment_eq(
            r#"
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_impls_with_data() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer").unwrap();
        builder.add_datum::<u32, _>("not_copy_integer").unwrap();
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(FromPreviousRecordUnnamedFieldsImplsGenerator)
                as Box<dyn FragmentGenerator>]);

        let mut scope = Scope::new();
        let mut type_size_assertions = BTreeSet::new();

        generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().next().expect("variant"),
            None,
            &config,
            &mut scope,
            &mut type_size_assertions,
        );

        assert_fragment_eq(
            r#"
"#,
            &scope.to_string(),
        );

        assert_eq!(
            btreeset![("u32", std::mem::size_of::<u32>())],
            type_size_assertions
        );
    }

    #[test]
    fn should_generate_next_impls_with_data() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        let i0 = builder
            .add_datum_allow_uninit::<u32, _>("integer0")
            .unwrap();
        let nci0 = builder.add_datum::<u32, _>("not_copy_integer0").unwrap();
        builder
            .add_datum_allow_uninit::<bool, _>("boolean1")
            .unwrap();
        builder.close_record_variant();
        builder.remove_datum(i0).unwrap();
        builder.remove_datum(nci0).unwrap();
        builder
            .add_datum_allow_uninit::<u32, _>("integer1")
            .unwrap();
        builder.add_datum::<u32, _>("not_copy_integer1").unwrap();
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(FromPreviousRecordUnnamedFieldsImplsGenerator)
                as Box<dyn FragmentGenerator>]);

        let mut scope = Scope::new();
        let mut type_size_assertions = BTreeSet::new();

        let record0_spec = generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().next().expect("variant"),
            None,
            &config,
            &mut scope,
            &mut type_size_assertions,
        );
        let mut scope = Scope::new();
        type_size_assertions.clear();
        generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().nth(1).expect("variant"),
            Some(&record0_spec),
            &config,
            &mut scope,
            &mut type_size_assertions,
        );

        assert_fragment_eq(
            r#"
impl<const CAP: usize> From<(CappedRecord0<CAP>, (u32, u32))> for CappedRecord1<CAP> {
    fn from((from, (integer1, not_copy_integer1)): (CappedRecord0<CAP>, (u32, u32))) -> Self {
        let _integer0: u32 = unsafe { from.data.read(0) };
        let _not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let plus = UnpackedRecordIn1 { integer1, not_copy_integer1 };
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let mut data = unsafe { std::ptr::read(&manually_drop.data) };
        unsafe { data.write(0, plus.integer1); }
        unsafe { data.write(4, plus.not_copy_integer1); }
        Self { data }
    }
}

impl<const CAP: usize> From<(CappedRecord0<CAP>, u32)> for CappedRecord1<CAP> {
    fn from((from, not_copy_integer1): (CappedRecord0<CAP>, u32)) -> Self {
        let _integer0: u32 = unsafe { from.data.read(0) };
        let _not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let plus = UnpackedUninitSafeRecordIn1::<u32>::from(UnpackedUninitRecordIn1 { not_copy_integer1 });
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let mut data = unsafe { std::ptr::read(&manually_drop.data) };
        unsafe { data.write(4, plus.not_copy_integer1); }
        Self { data }
    }
}

impl<const CAP: usize> From<(CappedRecord0<CAP>, (u32, u32))> for Record1AndUnpackedOut<CAP> {
    fn from((from, (integer1, not_copy_integer1)): (CappedRecord0<CAP>, (u32, u32))) -> Self {
        let integer0: u32 = unsafe { from.data.read(0) };
        let not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let plus = UnpackedRecordIn1 { integer1, not_copy_integer1 };
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let mut data = unsafe { std::ptr::read(&manually_drop.data) };
        unsafe { data.write(0, plus.integer1); }
        unsafe { data.write(4, plus.not_copy_integer1); }
        let record = CappedRecord1 { data };
        Record1AndUnpackedOut { record, integer0, not_copy_integer0 }
    }
}

impl<const CAP: usize> From<(CappedRecord0<CAP>, u32)> for Record1AndUnpackedOut<CAP> {
    fn from((from, not_copy_integer1): (CappedRecord0<CAP>, u32)) -> Self {
        let integer0: u32 = unsafe { from.data.read(0) };
        let not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let plus = UnpackedUninitSafeRecordIn1::<u32>::from(UnpackedUninitRecordIn1 { not_copy_integer1 });
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let mut data = unsafe { std::ptr::read(&manually_drop.data) };
        unsafe { data.write(4, plus.not_copy_integer1); }
        let record = CappedRecord1 { data };
        Record1AndUnpackedOut { record, integer0, not_copy_integer0 }
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(
            btreeset![("u32", std::mem::size_of::<u32>())],
            type_size_assertions
        );
    }

    #[test]
    fn should_generate_next_impls_with_only_removed_data() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        let i0 = builder
            .add_datum_allow_uninit::<u32, _>("integer0")
            .unwrap();
        let nci0 = builder.add_datum::<u32, _>("not_copy_integer0").unwrap();
        builder
            .add_datum_allow_uninit::<bool, _>("boolean1")
            .unwrap();
        builder.close_record_variant();
        builder.remove_datum(i0).unwrap();
        builder.remove_datum(nci0).unwrap();
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(FromPreviousRecordUnnamedFieldsImplsGenerator)
                as Box<dyn FragmentGenerator>]);

        let mut scope = Scope::new();
        let mut type_size_assertions = BTreeSet::new();

        let record0_spec = generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().next().expect("variant"),
            None,
            &config,
            &mut scope,
            &mut type_size_assertions,
        );
        let mut scope = Scope::new();
        type_size_assertions.clear();
        generate_variant(
            &definition,
            definition.max_type_align(),
            definition.variants().nth(1).expect("variant"),
            Some(&record0_spec),
            &config,
            &mut scope,
            &mut type_size_assertions,
        );

        assert_fragment_eq(
            r#"
impl<const CAP: usize> From<(CappedRecord0<CAP>, ())> for CappedRecord1<CAP> {
    fn from((from, ()): (CappedRecord0<CAP>, ())) -> Self {
        let _integer0: u32 = unsafe { from.data.read(0) };
        let _not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let data = unsafe { std::ptr::read(&manually_drop.data) };
        Self { data }
    }
}

impl<const CAP: usize> From<(CappedRecord0<CAP>, ())> for Record1AndUnpackedOut<CAP> {
    fn from((from, ()): (CappedRecord0<CAP>, ())) -> Self {
        let integer0: u32 = unsafe { from.data.read(0) };
        let not_copy_integer0: u32 = unsafe { from.data.read(4) };
        let manually_drop = std::mem::ManuallyDrop::new(from);
        let data = unsafe { std::ptr::read(&manually_drop.data) };
        let record = CappedRecord1 { data };
        Record1AndUnpackedOut { record, integer0, not_copy_integer0 }
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }
}
