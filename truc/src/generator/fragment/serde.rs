use codegen::{Function, Scope};
use itertools::Itertools;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::generator::{CAP, CAP_GENERIC};

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
