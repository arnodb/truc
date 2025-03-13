use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{generate_data_out_record, generate_data_record, RecordInfo, UninitKind};

pub struct FromPreviousRecordDataRecordsGenerator;

impl FragmentGenerator for FromPreviousRecordDataRecordsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let prev_record_spec = if let Some(prev_record_spec) = specs.prev_record.as_ref() {
            prev_record_spec
        } else {
            return;
        };
        let record_spec = &specs.record;

        for (record_info, uninit_kind) in [
            (
                RecordInfo {
                    name: &record_spec.unpacked_record_in_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`]."#,
                        prev_record_spec.variant.id()
                    )),
                },
                UninitKind::False,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_record_in_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`] without the data to be left uninitialized."#,
                        prev_record_spec.variant.id()
                    )),
                },
                UninitKind::Unsafe,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_safe_record_in_name,
                    public: false,
                    doc: Some(
                        r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                    ),
                },
                UninitKind::Safe {
                    unsafe_record_name: &record_spec.unpacked_uninit_record_in_name,
                    safe_generic: record_spec.plus_uninit_safe_generic.as_ref(),
                },
            ),
        ] {
            generate_data_record(record_info, &record_spec.plus_data, uninit_kind, scope);
        }

        generate_data_out_record(
            RecordInfo {
                name: &record_spec.record_and_unpacked_out_name,
                public: true,
                doc: Some(&format!(
                    r#"Result of conversion from record variant #{} to variant #{} via a [`From::from`] call.

It contains all the removed data so that one can still use them, or drop them."#,
                    prev_record_spec.variant.id(),
                    record_spec.variant.id()
                )),
            },
            &record_spec.capped_record_name,
            &record_spec.minus_data,
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
    fn should_generate_empty_data_records() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config = GeneratorConfig::new([
            Box::new(FromPreviousRecordDataRecordsGenerator) as Box<dyn FragmentGenerator>
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
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_data_records_with_data() {
        let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer").unwrap();
        builder.add_datum::<u32, _>("not_copy_integer").unwrap();
        builder.close_record_variant();
        let definition = builder.build();

        let config = GeneratorConfig::new([
            Box::new(FromPreviousRecordDataRecordsGenerator) as Box<dyn FragmentGenerator>
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
"#,
            &scope.to_string(),
        );

        assert_eq!(
            btreeset![("u32", std::mem::size_of::<u32>())],
            type_size_assertions
        );
    }

    #[test]
    fn should_generate_next_data_records_with_data() {
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
            Box::new(FromPreviousRecordDataRecordsGenerator) as Box<dyn FragmentGenerator>
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
/// Data container for conversion from [`Record0`].
pub struct UnpackedRecordIn1 {
    pub integer1: u32,
    pub not_copy_integer1: u32,
}

/// Data container for conversion from [`Record0`] without the data to be left uninitialized.
pub struct UnpackedUninitRecordIn1 {
    pub not_copy_integer1: u32,
}

/// It only exists to check that the uninitialized data is actually [`Copy`] at run time.
struct UnpackedUninitSafeRecordIn1<T0: Copy> {
    pub integer1: std::marker::PhantomData<T0>,
    pub not_copy_integer1: u32,
}

impl<T0: Copy> From<UnpackedUninitRecordIn1> for UnpackedUninitSafeRecordIn1<T0> {
    fn from(from: UnpackedUninitRecordIn1) -> Self {
        Self { integer1: std::marker::PhantomData, not_copy_integer1: from.not_copy_integer1 }
    }
}

/// Result of conversion from record variant #0 to variant #1 via a [`From::from`] call.
///
/// It contains all the removed data so that one can still use them, or drop them.
pub struct Record1AndUnpackedOut<const CAP: usize> {
    pub record: CappedRecord1<CAP>,
    pub integer0: u32,
    pub not_copy_integer0: u32,
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
