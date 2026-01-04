use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{fragment::RecordSpec, CAP, CAP_GENERIC};

pub struct FromUnnamedFieldsImplsGenerator;

impl FromUnnamedFieldsImplsGenerator {
    fn generate_from_unnamed_fields_impl(
        record_spec: &RecordSpec,
        uninit: bool,
        scope: &mut Scope,
    ) {
        if uninit
            && !record_spec
                .data
                .iter()
                .any(|datum| datum.details().allow_uninit())
        {
            return;
        }

        let (from_args, from_types) = if !uninit {
            let iter = record_spec.data.iter();
            (
                iter.clone()
                    .map(|datum| datum.name())
                    .collect::<Vec<&str>>(),
                iter.map(|datum| datum.details().type_name())
                    .collect::<Vec<&str>>(),
            )
        } else {
            let iter = record_spec
                .data
                .iter()
                .filter(|datum| !datum.details().allow_uninit());

            (
                iter.clone()
                    .map(|datum| datum.name())
                    .collect::<Vec<&str>>(),
                iter.map(|datum| datum.details().type_name())
                    .collect::<Vec<&str>>(),
            )
        };

        let ty = match from_types.len() {
            0 => "()".to_owned(),
            1 => from_types[0].to_owned(),
            _ => format!("({})", from_types.join(", ")),
        };

        let from_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait(format!("From<{}>", ty));

        let from_fn = from_impl
            .new_fn("from")
            .arg(
                &match from_args.len() {
                    0 => "()".to_owned(),
                    1 => from_args[0].to_owned(),
                    _ => format!("({})", from_args.join(", ")),
                },
                ty,
            )
            .ret("Self");
        from_fn.line(format!(
            "Self::{}({})",
            if !uninit {
                "new_unnamed"
            } else {
                "new_uninit_unnamed"
            },
            from_args.join(", "),
        ));
    }
}

impl FragmentGenerator for FromUnnamedFieldsImplsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        Self::generate_from_unnamed_fields_impl(record_spec, false, scope);

        Self::generate_from_unnamed_fields_impl(record_spec, true, scope);
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
            Box::new(FromUnnamedFieldsImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<()> for CappedRecord0<CAP> {
    fn from((): ()) -> Self {
        Self::new_unnamed()
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
            Box::new(FromUnnamedFieldsImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<(u32, u32)> for CappedRecord0<CAP> {
    fn from((integer, not_copy_integer): (u32, u32)) -> Self {
        Self::new_unnamed(integer, not_copy_integer)
    }
}

impl<const CAP: usize> From<u32> for CappedRecord0<CAP> {
    fn from(not_copy_integer: u32) -> Self {
        Self::new_uninit_unnamed(not_copy_integer)
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
            Box::new(FromUnnamedFieldsImplsGenerator) as Box<dyn FragmentGenerator>
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
impl<const CAP: usize> From<(bool, u32, u32)> for CappedRecord1<CAP> {
    fn from((boolean1, integer1, not_copy_integer1): (bool, u32, u32)) -> Self {
        Self::new_unnamed(boolean1, integer1, not_copy_integer1)
    }
}

impl<const CAP: usize> From<u32> for CappedRecord1<CAP> {
    fn from(not_copy_integer1: u32) -> Self {
        Self::new_uninit_unnamed(not_copy_integer1)
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
