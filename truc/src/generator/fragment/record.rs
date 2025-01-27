use codegen::{Scope, Type};

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{CAP, CAP_GENERIC};

pub struct RecordGenerator;

impl FragmentGenerator for RecordGenerator {
    fn imports(&self, scope: &mut Scope) {
        scope.import("truc_runtime::data", "RecordMaybeUninit");
    }

    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let record = scope
            .new_struct(&record_spec.capped_record_name)
            .repr(&format!("align({})", record_spec.max_type_align))
            .vis("pub")
            .generic(CAP_GENERIC);

        let mut uninit_type = Type::new("RecordMaybeUninit");
        uninit_type.generic(CAP);
        record.field("data", &uninit_type);

        if let Some(prev_record_spec) = specs.prev_record {
            record.doc(&format!(
                r#"Record variant #{}.

It may be converted from a [`Record{}`] via one of the various call to [`From::from`]

It may also be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                        record_spec.variant.id(),
                        prev_record_spec.variant.id()
                    ));
        } else {
            record.doc(&format!(
                r#"Record variant #{}.

It may be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                        record_spec.variant.id()
                    ));
        }

        scope.raw(format!(
            r#"/// Record variant #{} with optimized capacity.
pub type {} = {}<{{ MAX_SIZE }}>;"#,
            record_spec.variant.id(),
            record_spec.record_name,
            record_spec.capped_record_name,
        ));
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
    fn should_generate_empty_record() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(RecordGenerator) as Box<dyn FragmentGenerator>]);

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
use truc_runtime::data::RecordMaybeUninit;

/// Record variant #0.
///
/// It may be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)
#[repr(align(1))]
pub struct CappedRecord0<const CAP: usize> {
    data: RecordMaybeUninit<CAP>,
}

/// Record variant #0 with optimized capacity.
pub type Record0 = CappedRecord0<{ MAX_SIZE }>;
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_record_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer");
        builder.add_datum::<u32, _>("not_copy_integer");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(RecordGenerator) as Box<dyn FragmentGenerator>]);

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
use truc_runtime::data::RecordMaybeUninit;

/// Record variant #0.
///
/// It may be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)
#[repr(align(4))]
pub struct CappedRecord0<const CAP: usize> {
    data: RecordMaybeUninit<CAP>,
}

/// Record variant #0 with optimized capacity.
pub type Record0 = CappedRecord0<{ MAX_SIZE }>;
"#,
            &scope.to_string(),
        );

        assert_eq!(
            btreeset![("u32", std::mem::size_of::<u32>())],
            type_size_assertions
        );
    }

    #[test]
    fn should_generate_next_record_with_data() {
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
            GeneratorConfig::new([Box::new(RecordGenerator) as Box<dyn FragmentGenerator>]);

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
use truc_runtime::data::RecordMaybeUninit;

/// Record variant #1.
///
/// It may be converted from a [`Record0`] via one of the various call to [`From::from`]
///
/// It may also be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)
#[repr(align(4))]
pub struct CappedRecord1<const CAP: usize> {
    data: RecordMaybeUninit<CAP>,
}

/// Record variant #1 with optimized capacity.
pub type Record1 = CappedRecord1<{ MAX_SIZE }>;
"#,
            &scope.to_string(),
        );

        assert_eq!(
            btreeset![("u32", std::mem::size_of::<u32>())],
            type_size_assertions
        );
    }
}
