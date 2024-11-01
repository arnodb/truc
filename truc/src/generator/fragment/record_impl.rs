use std::ops::Deref;

use codegen::{Impl, Scope};
use itertools::Itertools;

use super::{FragmentGenerator, FragmentGeneratorSpecs, RecordSpec};
use crate::{
    generator::{CAP, CAP_GENERIC},
    record::definition::DatumDefinition,
};

#[derive(Debug)]
enum UninitKind<'a> {
    Full,
    Uninit { safe_record_name: &'a str },
}

pub struct RecordImplGenerator;

impl RecordImplGenerator {
    fn generate_constructor(
        record_spec: &RecordSpec,
        unpacked_record_name: &str,
        uninit_kind: UninitKind,
        record_impl: &mut Impl,
    ) {
        let has_data = !record_spec.data.is_empty();
        let new_fn = record_impl
            .new_fn(match uninit_kind {
                UninitKind::Full => "new",
                UninitKind::Uninit { .. } => "new_uninit",
            })
            .vis("pub")
            .arg(
                match (&uninit_kind, has_data) {
                    (UninitKind::Full, false) => "_from",
                    (UninitKind::Full, true) | (UninitKind::Uninit { .. }, _) => "from",
                },
                unpacked_record_name,
            )
            .ret("Self");
        let uninit_has_data = if let UninitKind::Uninit { safe_record_name } = uninit_kind {
            let uninit_has_data = record_spec.data.iter().any(|datum| !datum.allow_uninit());
            new_fn.line(format!(
                "let {} = {}{}::from(from);",
                if uninit_has_data { "from" } else { "_from" },
                safe_record_name,
                record_spec
                    .unpacked_uninit_safe_generic
                    .as_ref()
                    .map_or_else(String::new, |generic| format!("::<{}>", generic.typed))
            ));
            uninit_has_data
        } else {
            false
        };
        new_fn.line(format!(
            "let {}data = RecordMaybeUninit::new();",
            match (&uninit_kind, has_data, uninit_has_data) {
                (UninitKind::Full, true, _) | (UninitKind::Uninit { .. }, _, true) => "mut ",
                _ => "",
            },
        ));
        for datum in record_spec
            .data
            .iter()
            .filter(|datum| matches!(uninit_kind, UninitKind::Full) || !datum.allow_uninit())
        {
            new_fn.line(format!(
                "unsafe {{ data.write({}, from.{}); }}",
                datum.offset(),
                datum.name()
            ));
        }
        new_fn.line("Self { data }");
    }

    fn generate_unpacker(
        data: &[&DatumDefinition],
        unpacked_record_name: &str,
        record_impl: &mut Impl,
    ) {
        let unpack_fn = record_impl
            .new_fn("unpack")
            .arg_self()
            .vis("pub")
            .ret(unpacked_record_name);
        for datum in data {
            unpack_fn.line(format!(
                "let {}: {} = unsafe {{ self.data.read({}) }};",
                datum.name(),
                datum.type_name(),
                datum.offset(),
            ));
        }
        unpack_fn.line("std::mem::forget(self);");
        unpack_fn.line(format!(
            "{} {{ {} }}",
            unpacked_record_name,
            data.iter()
                .map(Deref::deref)
                .map(DatumDefinition::name)
                .join(", ")
        ));
    }
}

impl FragmentGenerator for RecordImplGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let record_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP);

        Self::generate_constructor(
            record_spec,
            &record_spec.unpacked_record_name,
            UninitKind::Full,
            record_impl,
        );

        Self::generate_constructor(
            record_spec,
            &record_spec.unpacked_uninit_record_name,
            UninitKind::Uninit {
                safe_record_name: &record_spec.unpacked_uninit_safe_record_name,
            },
            record_impl,
        );

        Self::generate_unpacker(
            &record_spec.data,
            &record_spec.unpacked_record_name,
            record_impl,
        );

        for datum in &record_spec.data {
            record_impl
                .new_fn(datum.name())
                .vis("pub")
                .arg_ref_self()
                .ret(format!("&{}", datum.type_name()))
                .line(format!(
                    "unsafe {{ self.data.get::<{}>({}) }}",
                    datum.type_name(),
                    datum.offset()
                ));

            record_impl
                .new_fn(&format!("{}_mut", datum.name()))
                .vis("pub")
                .arg_mut_self()
                .ret(format!("&mut {}", datum.type_name()))
                .line(format!(
                    "unsafe {{ self.data.get_mut::<{}>({}) }}",
                    datum.type_name(),
                    datum.offset()
                ));
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
    fn should_generate_empty_record_impl() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(RecordImplGenerator) as Box<dyn FragmentGenerator>]);

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
    pub fn new(_from: UnpackedRecord0) -> Self {
        let data = RecordMaybeUninit::new();
        Self { data }
    }

    pub fn new_uninit(from: UnpackedUninitRecord0) -> Self {
        let _from = UnpackedUninitSafeRecord0::from(from);
        let data = RecordMaybeUninit::new();
        Self { data }
    }

    pub fn unpack(self) -> UnpackedRecord0 {
        std::mem::forget(self);
        UnpackedRecord0 {  }
    }
}
"#,
            &scope.to_string(),
        );

        assert_eq!(btreeset![], type_size_assertions);
    }

    #[test]
    fn should_generate_record_impl_with_data() {
        let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
        builder.add_datum_allow_uninit::<u32, _>("integer");
        builder.add_datum::<u32, _>("not_copy_integer");
        builder.close_record_variant();
        let definition = builder.build();

        let config =
            GeneratorConfig::new([Box::new(RecordImplGenerator) as Box<dyn FragmentGenerator>]);

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
    pub fn new(from: UnpackedRecord0) -> Self {
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(0, from.integer); }
        unsafe { data.write(4, from.not_copy_integer); }
        Self { data }
    }

    pub fn new_uninit(from: UnpackedUninitRecord0) -> Self {
        let from = UnpackedUninitSafeRecord0::<u32>::from(from);
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(4, from.not_copy_integer); }
        Self { data }
    }

    pub fn unpack(self) -> UnpackedRecord0 {
        let integer: u32 = unsafe { self.data.read(0) };
        let not_copy_integer: u32 = unsafe { self.data.read(4) };
        std::mem::forget(self);
        UnpackedRecord0 { integer, not_copy_integer }
    }

    pub fn integer(&self) -> &u32 {
        unsafe { self.data.get::<u32>(0) }
    }

    pub fn integer_mut(&mut self) -> &mut u32 {
        unsafe { self.data.get_mut::<u32>(0) }
    }

    pub fn not_copy_integer(&self) -> &u32 {
        unsafe { self.data.get::<u32>(4) }
    }

    pub fn not_copy_integer_mut(&mut self) -> &mut u32 {
        unsafe { self.data.get_mut::<u32>(4) }
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
            GeneratorConfig::new([Box::new(RecordImplGenerator) as Box<dyn FragmentGenerator>]);

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
    pub fn new(from: UnpackedRecord1) -> Self {
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(8, from.boolean1); }
        unsafe { data.write(0, from.integer1); }
        unsafe { data.write(4, from.not_copy_integer1); }
        Self { data }
    }

    pub fn new_uninit(from: UnpackedUninitRecord1) -> Self {
        let from = UnpackedUninitSafeRecord1::<bool, u32>::from(from);
        let mut data = RecordMaybeUninit::new();
        unsafe { data.write(4, from.not_copy_integer1); }
        Self { data }
    }

    pub fn unpack(self) -> UnpackedRecord1 {
        let boolean1: bool = unsafe { self.data.read(8) };
        let integer1: u32 = unsafe { self.data.read(0) };
        let not_copy_integer1: u32 = unsafe { self.data.read(4) };
        std::mem::forget(self);
        UnpackedRecord1 { boolean1, integer1, not_copy_integer1 }
    }

    pub fn boolean1(&self) -> &bool {
        unsafe { self.data.get::<bool>(8) }
    }

    pub fn boolean1_mut(&mut self) -> &mut bool {
        unsafe { self.data.get_mut::<bool>(8) }
    }

    pub fn integer1(&self) -> &u32 {
        unsafe { self.data.get::<u32>(0) }
    }

    pub fn integer1_mut(&mut self) -> &mut u32 {
        unsafe { self.data.get_mut::<u32>(0) }
    }

    pub fn not_copy_integer1(&self) -> &u32 {
        unsafe { self.data.get::<u32>(4) }
    }

    pub fn not_copy_integer1_mut(&mut self) -> &mut u32 {
        unsafe { self.data.get_mut::<u32>(4) }
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
