use codegen::{Impl, Scope};
use itertools::Itertools;
use tap::Pipe;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::generator::{CAP, CAP_GENERIC};

#[derive(Debug)]
enum UninitKind<'a> {
    Full,
    Uninit { safe_record_name: &'a str },
}

pub struct RecordUnnamedImplGenerator;

impl RecordUnnamedImplGenerator {
    fn generate_constructor(
        record_spec: &RecordSpec,
        uninit_kind: UninitKind,
        record_impl: &mut Impl,
    ) {
        let has_data = !record_spec.data.is_empty();
        let new_fn = record_impl
            .new_fn(match uninit_kind {
                UninitKind::Full => "new_unnamed",
                UninitKind::Uninit { .. } => "new_uninit_unnamed",
            })
            .vis("pub")
            .pipe(|f| {
                record_spec.data.iter().fold(f, |f, datum| {
                    match (&uninit_kind, datum.details().allow_uninit()) {
                        (UninitKind::Full, _) | (UninitKind::Uninit { .. }, false) => {
                            f.arg(datum.name(), datum.details().type_name())
                        }
                        (UninitKind::Uninit { .. }, true) => f,
                    }
                })
            })
            .ret("Self");
        let uninit_has_data = match uninit_kind {
            UninitKind::Full => {
                new_fn.line(format!(
                    "let {} = {} {{ {} }};",
                    match has_data {
                        false => "_from",
                        true => "from",
                    },
                    &record_spec.unpacked_record_name,
                    record_spec.data.iter().map(|datum| datum.name()).join(", ")
                ));
                false
            }
            UninitKind::Uninit { safe_record_name } => {
                let uninit_has_data = record_spec
                    .data
                    .iter()
                    .any(|datum| !datum.details().allow_uninit());
                new_fn.line(format!(
                    "let {} = {}{}::from({} {{ {} }});",
                    if uninit_has_data { "from" } else { "_from" },
                    safe_record_name,
                    record_spec
                        .unpacked_uninit_safe_generic
                        .as_ref()
                        .map_or_else(String::new, |generic| format!("::<{}>", generic.typed)),
                    record_spec.unpacked_uninit_record_name,
                    record_spec
                        .data
                        .iter()
                        .filter(|datum| { !datum.details().allow_uninit() })
                        .map(|datum| datum.name())
                        .join(", ")
                ));
                uninit_has_data
            }
        };
        new_fn.line(format!(
            "let {}data = RecordMaybeUninit::new();",
            match (&uninit_kind, has_data, uninit_has_data) {
                (UninitKind::Full, true, _) | (UninitKind::Uninit { .. }, _, true) => "mut ",
                _ => "",
            },
        ));
        for datum in record_spec.data.iter().filter(|datum| {
            matches!(uninit_kind, UninitKind::Full) || !datum.details().allow_uninit()
        }) {
            new_fn.line(format!(
                "unsafe {{ data.write({}, from.{}); }}",
                datum.details().offset(),
                datum.name()
            ));
        }
        new_fn.line("Self { data }");
    }
}

impl FragmentGenerator for RecordUnnamedImplGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let record_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP);

        Self::generate_constructor(record_spec, UninitKind::Full, record_impl);

        Self::generate_constructor(
            record_spec,
            UninitKind::Uninit {
                safe_record_name: &record_spec.unpacked_uninit_safe_record_name,
            },
            record_impl,
        );
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
    fn should_generate_empty_record_impl() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config = GeneratorConfig::new([
            Box::new(RecordUnnamedImplGenerator) as Box<dyn FragmentGenerator>
        ]);

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
impl<const CAP: usize> CappedRecord0<CAP> {
    pub fn new_unnamed() -> Self {
        let _from = UnpackedRecord0 {};
        let data = RecordMaybeUninit::new();
        Self { data }
    }

    pub fn new_uninit_unnamed() -> Self {
        let _from = UnpackedUninitSafeRecord0::from(UnpackedUninitRecord0 {});
        let data = RecordMaybeUninit::new();
        Self { data }
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_record_impl_with_data() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer").unwrap();
        builder.add_datum::<u32, _>("not_copy_integer").unwrap();
        builder.close_record_variant();
        let definition = builder.build();

        let config = GeneratorConfig::new([
            Box::new(RecordUnnamedImplGenerator) as Box<dyn FragmentGenerator>
        ]);

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
impl<const CAP: usize> CappedRecord0<CAP> {
    pub fn new_unnamed(integer: u32, not_copy_integer: u32) -> Self {
        let from = UnpackedRecord0 { integer, not_copy_integer };
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(0, from.integer); }
        unsafe { data.write(4, from.not_copy_integer); }
        Self { data }
    }

    pub fn new_uninit_unnamed(not_copy_integer: u32) -> Self {
        let from = UnpackedUninitSafeRecord0::<u32>::from(UnpackedUninitRecord0 { not_copy_integer });
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(4, from.not_copy_integer); }
        Self { data }
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
    fn should_generate_next_record_impl_with_data() {
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

        let config = GeneratorConfig::new([
            Box::new(RecordUnnamedImplGenerator) as Box<dyn FragmentGenerator>
        ]);

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
impl<const CAP: usize> CappedRecord1<CAP> {
    pub fn new_unnamed(boolean1: bool, integer1: u32, not_copy_integer1: u32) -> Self {
        let from = UnpackedRecord1 { boolean1, integer1, not_copy_integer1 };
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(8, from.boolean1); }
        unsafe { data.write(0, from.integer1); }
        unsafe { data.write(4, from.not_copy_integer1); }
        Self { data }
    }

    pub fn new_uninit_unnamed(not_copy_integer1: u32) -> Self {
        let from = UnpackedUninitSafeRecord1::<bool, u32>::from(UnpackedUninitRecord1 { not_copy_integer1 });
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(4, from.not_copy_integer1); }
        Self { data }
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
}
