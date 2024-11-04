//! Serialization features.

use codegen::{Function, Scope};
use itertools::Itertools;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::generator::{CAP, CAP_GENERIC};

/// Use this generator in [GeneratorConfig](crate::generator::config::GeneratorConfig) in order to
/// enable serialization features on generated structures.
pub struct SerdeImplGenerator;

impl SerdeImplGenerator {
    fn generate_serialize_impl(record_spec: &RecordSpec, scope: &mut Scope) {
        let serialize_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait("serde::Serialize");

        let serialize_fn = serialize_impl
            .new_fn("serialize")
            .generic("S")
            .arg_ref_self()
            .arg("serializer", "S")
            .ret("Result<S::Ok, S::Error>")
            .bound("S", "serde::Serializer");

        if !record_spec.data.is_empty() {
            serialize_fn.line(format!(
                "let mut tuple = serializer.serialize_tuple({})?;",
                record_spec.data.len()
            ));
        } else {
            serialize_fn.line("let tuple = serializer.serialize_tuple(0)?;");
        }
        for datum in &record_spec.data {
            serialize_fn.line(format!(
                "tuple.serialize_element(self.{}())?;",
                datum.name()
            ));
        }
        serialize_fn.line("tuple.end()");
    }

    fn generate_visitor(record_spec: &RecordSpec, deserialize_fn: &mut Function) {
        let mut sub_scope = Scope::new();

        sub_scope.new_struct("RecordVisitor").generic(CAP_GENERIC);

        let visitor_impl = sub_scope
            .new_impl(&format!("RecordVisitor<{}>", CAP))
            .generic(&format!("'de, {}", CAP_GENERIC))
            .impl_trait("serde::de::Visitor<'de>");

        visitor_impl.associate_type(
            "Value",
            format!("{}<{{ {} }}>", &record_spec.capped_record_name, CAP),
        );

        visitor_impl
            .new_fn("expecting")
            .arg_ref_self()
            .arg("formatter", "&mut std::fmt::Formatter")
            .ret("std::fmt::Result")
            .line(format!(
                "formatter.write_str(\"a {}\")",
                record_spec.capped_record_name
            ));

        let visit_seq_fn = visitor_impl
            .new_fn("visit_seq")
            .generic("A")
            .arg_self()
            .arg(
                if !record_spec.data.is_empty() {
                    "mut seq"
                } else {
                    "seq"
                },
                "A",
            )
            .ret("Result<Self::Value, A::Error>")
            .bound("A", "serde::de::SeqAccess<'de>");

        visit_seq_fn.line("if let Some(size) = seq.size_hint() {");
        visit_seq_fn.line(format!(
            "    if size != {} {{ return Err(A::Error::invalid_length(size, &\"{}\")); }}",
            record_spec.data.len(),
            record_spec.data.len(),
        ));
        visit_seq_fn.line("}");

        for datum in &record_spec.data {
            visit_seq_fn.line(format!(
                "let {} = seq.next_element::<{}>()?.ok_or_else(|| A::Error::missing_field(\"{}\"))?;",
                datum.name(),
                datum.type_name(),
                datum.name(),
            ));
        }

        visit_seq_fn.line("if let Some(size) = seq.size_hint() {");
        visit_seq_fn.line("    assert_eq!(size, 0);");
        visit_seq_fn.line("}");

        visit_seq_fn.line(format!(
            "Ok({}::new({} {{ {} }}))",
            record_spec.capped_record_name,
            record_spec.unpacked_record_name,
            record_spec.data.iter().map(|datum| datum.name()).join(", "),
        ));

        deserialize_fn.line(sub_scope.to_string());
        deserialize_fn.line("");
    }

    fn generate_deserialize_impl(record_spec: &RecordSpec, scope: &mut Scope) {
        let deserialize_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(&format!("'de, {}", CAP_GENERIC))
            .target_generic(CAP)
            .impl_trait("serde::Deserialize<'de>");

        let deserialize_fn = deserialize_impl
            .new_fn("deserialize")
            .generic("D")
            .arg("deserializer", "D")
            .ret("Result<Self, D::Error>")
            .bound("D", "serde::Deserializer<'de>");

        Self::generate_visitor(record_spec, deserialize_fn);

        deserialize_fn.line(&format!(
            "deserializer.deserialize_tuple({}, RecordVisitor::<{}>)",
            record_spec.data.len(),
            CAP
        ));
    }
}

impl FragmentGenerator for SerdeImplGenerator {
    fn imports(&self, scope: &mut Scope) {
        scope.import("serde::ser", "SerializeTuple");
        scope.import("serde::de", "Error");
    }

    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = specs.record;

        Self::generate_serialize_impl(record_spec, scope);

        Self::generate_deserialize_impl(record_spec, scope);
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
    fn should_generate_empty_serde_impl() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(SerdeImplGenerator) as Box<dyn FragmentGenerator>]);

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
use serde::ser::SerializeTuple;
use serde::de::Error;

impl<const CAP: usize> serde::Serialize for CappedRecord0<CAP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer,
    {
        let tuple = serializer.serialize_tuple(0)?;
        tuple.end()
    }
}

impl<'de, const CAP: usize> serde::Deserialize<'de> for CappedRecord0<CAP> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
    {
        struct RecordVisitor<const CAP: usize>;

        impl<'de, const CAP: usize> serde::de::Visitor<'de> for RecordVisitor<CAP> {
            type Value = CappedRecord0<{ CAP }>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CappedRecord0")
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de>,
            {
                if let Some(size) = seq.size_hint() {
                    if size != 0 { return Err(A::Error::invalid_length(size, &"0")); }
                }
                if let Some(size) = seq.size_hint() {
                    assert_eq!(size, 0);
                }
                Ok(CappedRecord0::new(UnpackedRecord0 {  }))
            }
        }

        deserializer.deserialize_tuple(0, RecordVisitor::<CAP>)
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_serde_impl_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer");
        builder.add_datum::<u32, _>("not_copy_integer");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(SerdeImplGenerator) as Box<dyn FragmentGenerator>]);

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
use serde::ser::SerializeTuple;
use serde::de::Error;

impl<const CAP: usize> serde::Serialize for CappedRecord0<CAP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.integer())?;
        tuple.serialize_element(self.not_copy_integer())?;
        tuple.end()
    }
}

impl<'de, const CAP: usize> serde::Deserialize<'de> for CappedRecord0<CAP> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
    {
        struct RecordVisitor<const CAP: usize>;

        impl<'de, const CAP: usize> serde::de::Visitor<'de> for RecordVisitor<CAP> {
            type Value = CappedRecord0<{ CAP }>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CappedRecord0")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de>,
            {
                if let Some(size) = seq.size_hint() {
                    if size != 2 { return Err(A::Error::invalid_length(size, &"2")); }
                }
                let integer = seq.next_element::<u32>()?.ok_or_else(|| A::Error::missing_field("integer"))?;
                let not_copy_integer = seq.next_element::<u32>()?.ok_or_else(|| A::Error::missing_field("not_copy_integer"))?;
                if let Some(size) = seq.size_hint() {
                    assert_eq!(size, 0);
                }
                Ok(CappedRecord0::new(UnpackedRecord0 { integer, not_copy_integer }))
            }
        }

        deserializer.deserialize_tuple(2, RecordVisitor::<CAP>)
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
    fn should_generate_next_serde_impl_with_data() {
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
            GeneratorConfig::new([Box::new(SerdeImplGenerator) as Box<dyn FragmentGenerator>]);

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
use serde::ser::SerializeTuple;
use serde::de::Error;

impl<const CAP: usize> serde::Serialize for CappedRecord1<CAP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(self.boolean1())?;
        tuple.serialize_element(self.integer1())?;
        tuple.serialize_element(self.not_copy_integer1())?;
        tuple.end()
    }
}

impl<'de, const CAP: usize> serde::Deserialize<'de> for CappedRecord1<CAP> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
    {
        struct RecordVisitor<const CAP: usize>;

        impl<'de, const CAP: usize> serde::de::Visitor<'de> for RecordVisitor<CAP> {
            type Value = CappedRecord1<{ CAP }>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CappedRecord1")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de>,
            {
                if let Some(size) = seq.size_hint() {
                    if size != 3 { return Err(A::Error::invalid_length(size, &"3")); }
                }
                let boolean1 = seq.next_element::<bool>()?.ok_or_else(|| A::Error::missing_field("boolean1"))?;
                let integer1 = seq.next_element::<u32>()?.ok_or_else(|| A::Error::missing_field("integer1"))?;
                let not_copy_integer1 = seq.next_element::<u32>()?.ok_or_else(|| A::Error::missing_field("not_copy_integer1"))?;
                if let Some(size) = seq.size_hint() {
                    assert_eq!(size, 0);
                }
                Ok(CappedRecord1::new(UnpackedRecord1 { boolean1, integer1, not_copy_integer1 }))
            }
        }

        deserializer.deserialize_tuple(3, RecordVisitor::<CAP>)
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
