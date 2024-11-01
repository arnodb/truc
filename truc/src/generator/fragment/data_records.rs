use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{generate_data_record, RecordInfo, UninitKind};

pub struct DataRecordsGenerator;

impl FragmentGenerator for DataRecordsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        for (record_info, uninit_kind) in [
            (
                RecordInfo {
                    name: &record_spec.unpacked_record_name,
                    public: true,
                    doc: Some(
                        r#"Data container for packing/unpacking records.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                    ),
                },
                UninitKind::False,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_record_name,
                    public: true,
                    doc: Some(
                        r#"Data container for packing/unpacking records without the data to be left uninitialized.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                    ),
                },
                UninitKind::Unsafe,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_safe_record_name,
                    public: false,
                    doc: Some(
                        r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                    ),
                },
                UninitKind::Safe {
                    unsafe_record_name: &record_spec.unpacked_uninit_record_name,
                    safe_generic: record_spec.unpacked_uninit_safe_generic.as_ref(),
                },
            ),
        ] {
            generate_data_record(record_info, &record_spec.data, uninit_kind, scope);
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
        record::{definition::RecordDefinitionBuilder, type_resolver::HostTypeResolver},
    };

    #[test]
    fn should_generate_empty_data_record() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(DataRecordsGenerator) as Box<dyn FragmentGenerator>]);

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
/// Data container for packing/unpacking records.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedRecord0;

/// Data container for packing/unpacking records without the data to be left uninitialized.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedUninitRecord0;

/// It only exists to check that the uninitialized data is actually [`Copy`] at run time.
struct UnpackedUninitSafeRecord0;

impl From<UnpackedUninitRecord0> for UnpackedUninitSafeRecord0 {
    fn from(_from: UnpackedUninitRecord0) -> Self {
        Self {  }
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_data_record_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer");
        builder.add_datum::<u32, _>("not_copy_integer");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(DataRecordsGenerator) as Box<dyn FragmentGenerator>]);

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
/// Data container for packing/unpacking records.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedRecord0 {
    pub integer: u32,
    pub not_copy_integer: u32,
}

/// Data container for packing/unpacking records without the data to be left uninitialized.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedUninitRecord0 {
    pub not_copy_integer: u32,
}

/// It only exists to check that the uninitialized data is actually [`Copy`] at run time.
struct UnpackedUninitSafeRecord0<T0: Copy> {
    pub integer: std::marker::PhantomData<T0>,
    pub not_copy_integer: u32,
}

impl<T0: Copy> From<UnpackedUninitRecord0> for UnpackedUninitSafeRecord0<T0> {
    fn from(from: UnpackedUninitRecord0) -> Self {
        Self { integer: std::marker::PhantomData, not_copy_integer: from.not_copy_integer }
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
    fn should_generate_next_data_record_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        let i0 = builder.add_datum_allow_uninit::<u32, _>("integer0");
        let nci0 = builder.add_datum::<u32, _>("not_copy_integer0");
        builder.add_datum_allow_uninit::<bool, _>("boolean1");
        builder.close_record_variant();
        builder.remove_datum(i0);
        builder.remove_datum(nci0);
        builder.add_datum_allow_uninit::<u32, _>("integer1");
        builder.add_datum::<u32, _>("not_copy_integer1");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(DataRecordsGenerator) as Box<dyn FragmentGenerator>]);

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
/// Data container for packing/unpacking records.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedRecord1 {
    pub boolean1: bool,
    pub integer1: u32,
    pub not_copy_integer1: u32,
}

/// Data container for packing/unpacking records without the data to be left uninitialized.
///
/// All the fields are named for the safe interoperability between the generated code and the code
/// using it.
pub struct UnpackedUninitRecord1 {
    pub not_copy_integer1: u32,
}

/// It only exists to check that the uninitialized data is actually [`Copy`] at run time.
struct UnpackedUninitSafeRecord1<T0: Copy, T1: Copy> {
    pub boolean1: std::marker::PhantomData<T0>,
    pub integer1: std::marker::PhantomData<T1>,
    pub not_copy_integer1: u32,
}

impl<T0: Copy, T1: Copy> From<UnpackedUninitRecord1> for UnpackedUninitSafeRecord1<T0, T1> {
    fn from(from: UnpackedUninitRecord1) -> Self {
        Self { boolean1: std::marker::PhantomData, integer1: std::marker::PhantomData, not_copy_integer1: from.not_copy_integer1 }
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
