use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{RecordImplRecordNames, CAP, CAP_GENERIC};

pub struct FromUnpackedRecordImplsGenerator;

impl FromUnpackedRecordImplsGenerator {
    fn generate_from_unpacked_record_impl(
        record_names: RecordImplRecordNames,
        uninit: bool,
        scope: &mut Scope,
    ) {
        let from_impl = scope
            .new_impl(record_names.name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait(format!("From<{}>", record_names.unpacked));

        let from_fn = from_impl
            .new_fn("from")
            .arg("from", record_names.unpacked)
            .ret("Self");
        from_fn.line(format!(
            "Self::{}(from)",
            if !uninit { "new" } else { "new_uninit" },
        ));
    }
}

impl FragmentGenerator for FromUnpackedRecordImplsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        Self::generate_from_unpacked_record_impl(
            RecordImplRecordNames {
                name: &record_spec.capped_record_name,
                unpacked: &record_spec.unpacked_record_name,
            },
            false,
            scope,
        );

        Self::generate_from_unpacked_record_impl(
            RecordImplRecordNames {
                name: &record_spec.capped_record_name,
                unpacked: &record_spec.unpacked_uninit_record_name,
            },
            true,
            scope,
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
    fn should_generate_empty_impls() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config = GeneratorConfig::new([
            Box::new(FromUnpackedRecordImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<UnpackedRecord0> for CappedRecord0<CAP> {
    fn from(from: UnpackedRecord0) -> Self {
        Self::new(from)
    }
}

impl<const CAP: usize> From<UnpackedUninitRecord0> for CappedRecord0<CAP> {
    fn from(from: UnpackedUninitRecord0) -> Self {
        Self::new_uninit(from)
    }
}
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

        let config = GeneratorConfig::new([
            Box::new(FromUnpackedRecordImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<UnpackedRecord0> for CappedRecord0<CAP> {
    fn from(from: UnpackedRecord0) -> Self {
        Self::new(from)
    }
}

impl<const CAP: usize> From<UnpackedUninitRecord0> for CappedRecord0<CAP> {
    fn from(from: UnpackedUninitRecord0) -> Self {
        Self::new_uninit(from)
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

        let config = GeneratorConfig::new([
            Box::new(FromUnpackedRecordImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<UnpackedRecord1> for CappedRecord1<CAP> {
    fn from(from: UnpackedRecord1) -> Self {
        Self::new(from)
    }
}

impl<const CAP: usize> From<UnpackedUninitRecord1> for CappedRecord1<CAP> {
    fn from(from: UnpackedUninitRecord1) -> Self {
        Self::new_uninit(from)
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
