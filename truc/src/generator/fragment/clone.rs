//! Clone support.

use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::generator::{CAP, CAP_GENERIC};

/// Use this generator in [GeneratorConfig](crate::generator::config::GeneratorConfig) in order to
/// generate `Clone` implementations.
pub struct CloneImplGenerator;

impl CloneImplGenerator {
    fn generate_clone_impl(record_spec: &RecordSpec, scope: &mut Scope) {
        let clone_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait("Clone");

        {
            let clone_fn = clone_impl.new_fn("clone").arg_ref_self().ret("Self");
            for datum in &record_spec.data {
                clone_fn.line(format!(
                    "let {} = self.{}().clone();",
                    datum.name(),
                    datum.name()
                ));
            }
            clone_fn.line(format!(
                "Self::from({} {{",
                record_spec.unpacked_record_name
            ));
            for datum in &record_spec.data {
                clone_fn.line(format!("    {},", datum.name()));
            }
            clone_fn.line("})");
        }

        {
            let clone_from_fn = clone_impl
                .new_fn("clone_from")
                .arg_mut_self()
                .arg("source", "&Self");
            for datum in &record_spec.data {
                clone_from_fn.line(format!(
                    "self.{}_mut().clone_from(source.{}());",
                    datum.name(),
                    datum.name()
                ));
            }
        }
    }
}

impl FragmentGenerator for CloneImplGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = specs.record;

        Self::generate_clone_impl(record_spec, scope);
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
    fn should_generate_empty_clone_impl() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(CloneImplGenerator) as Box<dyn FragmentGenerator>]);

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
impl<const CAP: usize> Clone for CappedRecord0<CAP> {
    fn clone(&self) -> Self {
        Self::from(UnpackedRecord0 {
        })
    }

    fn clone_from(&mut self, source: &Self) {
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_clone_impl_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer");
        builder.add_datum::<u32, _>("not_copy_integer");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(CloneImplGenerator) as Box<dyn FragmentGenerator>]);

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
impl<const CAP: usize> Clone for CappedRecord0<CAP> {
    fn clone(&self) -> Self {
        let integer = self.integer().clone();
        let not_copy_integer = self.not_copy_integer().clone();
        Self::from(UnpackedRecord0 {
            integer,
            not_copy_integer,
        })
    }

    fn clone_from(&mut self, source: &Self) {
        self.integer_mut().clone_from(source.integer());
        self.not_copy_integer_mut().clone_from(source.not_copy_integer());
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
    fn should_generate_next_clone_impl_with_data() {
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
            GeneratorConfig::new([Box::new(CloneImplGenerator) as Box<dyn FragmentGenerator>]);

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
impl<const CAP: usize> Clone for CappedRecord1<CAP> {
    fn clone(&self) -> Self {
        let boolean1 = self.boolean1().clone();
        let integer1 = self.integer1().clone();
        let not_copy_integer1 = self.not_copy_integer1().clone();
        Self::from(UnpackedRecord1 {
            boolean1,
            integer1,
            not_copy_integer1,
        })
    }

    fn clone_from(&mut self, source: &Self) {
        self.boolean1_mut().clone_from(source.boolean1());
        self.integer1_mut().clone_from(source.integer1());
        self.not_copy_integer1_mut().clone_from(source.not_copy_integer1());
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
