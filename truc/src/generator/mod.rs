//! See [GeneratorConfig] to customize the code generation.

use std::collections::BTreeSet;

use codegen::{Scope, Type};
use itertools::{Either, EitherOrBoth, Itertools};

use self::{
    config::GeneratorConfig,
    fragment::{FragmentGeneratorSpecs, RecordGeneric, RecordSpec},
};
use crate::record::definition::{DatumDefinition, RecordDefinition, RecordVariant};

pub mod config;
pub mod fragment;

const CAP_GENERIC: &str = "const CAP: usize";
const CAP: &str = "CAP";

/// Generates the code for the given record definition.
pub fn generate(definition: &RecordDefinition, config: &GeneratorConfig) -> String {
    let mut scope = Scope::new();

    scope.import("truc_runtime::data", "RecordMaybeUninit");

    let mut uninit_type = Type::new("RecordMaybeUninit");
    uninit_type.generic(CAP);

    let max_type_align = definition.max_type_align();
    let max_size = definition.max_size();

    scope.raw(format!(
        r#"/// Maximum size of the record, regardless of the record variant.
///
/// Use that value, or a greater value, as the `CAP` const generic of every record.
pub const MAX_SIZE: usize = {};"#,
        max_size
    ));

    let record_uninit = scope
        .new_struct("RecordUninitialized")
        .repr(&format!("align({})", max_type_align))
        .vis("pub")
        .generic(CAP_GENERIC);
    record_uninit.field("_data", &uninit_type);
    record_uninit.doc(
        r#"Uninitialized record.

It will never drop any data except the container by itself.

# Intention

This is to be used in custom allocators."#,
    );

    let mut prev_record_spec: Option<RecordSpec> = None;

    let mut type_size_assertions = BTreeSet::new();

    for variant in definition.variants() {
        let record_spec = generate_variant(
            definition,
            max_type_align,
            variant,
            prev_record_spec.as_ref(),
            config,
            &mut scope,
            &mut type_size_assertions,
        );

        prev_record_spec = Some(record_spec);
    }

    // This checks there is no type substitution which could lead to unsafe
    // code due to different type size.
    for (type_name, size) in type_size_assertions {
        scope.raw(format!(
            "const_assert_eq!(std::mem::size_of::<{}>(), {});",
            type_name, size
        ));
    }

    scope.to_string()
}

/// Generates the code for a given record variant.
///
/// This function is exposed for testing purpose.
pub fn generate_variant<'a>(
    definition: &'a RecordDefinition,
    max_type_align: usize,
    variant: &'a RecordVariant,
    prev_record_spec: Option<&RecordSpec>,
    config: &GeneratorConfig,
    scope: &mut Scope,
    type_size_assertions: &mut BTreeSet<(&'a str, usize)>,
) -> RecordSpec<'a> {
    let data = variant
        .data()
        .sorted()
        .map(|d| &definition[d])
        .collect::<Vec<_>>();
    let (minus_data, plus_data) = if let Some(prev_record_spec) = &prev_record_spec {
        prev_record_spec
            .variant
            .data()
            .sorted()
            .merge_join_by(&data, |left_id, right| left_id.cmp(&right.id()))
            .filter_map(|either| match either {
                EitherOrBoth::Left(left_id) => Some(Either::Left(&definition[left_id])),
                EitherOrBoth::Right(right) => Some(Either::Right(right)),
                EitherOrBoth::Both(_, _) => None,
            })
            .partition_map::<Vec<_>, Vec<_>, _, _, _>(|e| e)
    } else {
        (Vec::new(), data.clone())
    };
    let unpacked_uninit_safe_generic = safe_record_generic(&data);
    let plus_uninit_safe_generic = safe_record_generic(&plus_data);
    let record_spec = RecordSpec {
        max_type_align,
        variant,
        capped_record_name: format!("CappedRecord{}", variant.id()),
        record_name: format!("Record{}", variant.id()),
        unpacked_record_name: format!("UnpackedRecord{}", variant.id()),
        unpacked_uninit_record_name: format!("UnpackedUninitRecord{}", variant.id()),
        unpacked_uninit_safe_record_name: format!("UnpackedUninitSafeRecord{}", variant.id()),
        unpacked_record_in_name: format!("UnpackedRecordIn{}", variant.id()),
        unpacked_uninit_record_in_name: format!("UnpackedUninitRecordIn{}", variant.id()),
        unpacked_uninit_safe_record_in_name: format!("UnpackedUninitSafeRecordIn{}", variant.id()),
        record_and_unpacked_out_name: format!("Record{}AndUnpackedOut", variant.id()),
        data,
        minus_data,
        plus_data,
        unpacked_uninit_safe_generic,
        plus_uninit_safe_generic,
    };

    for datum in &record_spec.plus_data {
        type_size_assertions.insert((datum.type_name(), datum.size()));
    }

    let specs = FragmentGeneratorSpecs {
        record: &record_spec,
        prev_record: prev_record_spec,
    };

    let fragment_generators = config.fragment_generators.iter();

    for generator in fragment_generators {
        generator.imports(scope);
        generator.generate(&specs, scope);
    }

    record_spec
}

struct RecordImplRecordNames<'a> {
    name: &'a str,
    unpacked: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
enum UninitKind<'a> {
    False,
    Unsafe,
    Safe {
        unsafe_record_name: &'a str,
        safe_generic: Option<&'a RecordGeneric>,
    },
}

fn safe_record_generic(data: &[&DatumDefinition]) -> Option<RecordGeneric> {
    let mut generic = String::new();
    let mut short_generic = String::new();
    let mut typed_generic = String::new();
    for (index, datum) in data.iter().enumerate() {
        if datum.allow_uninit() {
            if !generic.is_empty() {
                generic.push_str(", ");
                short_generic.push_str(", ");
                typed_generic.push_str(", ");
            }
            generic.push_str(&format!("T{}: Copy", index));
            short_generic.push_str(&format!("T{}", index));
            typed_generic.push_str(datum.type_name());
        }
    }

    if !generic.is_empty() {
        Some(RecordGeneric {
            full: generic,
            short: short_generic,
            typed: typed_generic,
        })
    } else {
        None
    }
}

struct RecordInfo<'a> {
    name: &'a str,
    public: bool,
    doc: Option<&'a str>,
}

fn generate_data_record(
    record_info: RecordInfo,
    data: &[&DatumDefinition],
    uninit: UninitKind,
    scope: &mut Scope,
) {
    let record = scope.new_struct(record_info.name);
    if record_info.public {
        record.vis("pub");
    }

    if let UninitKind::Safe {
        unsafe_record_name: _,
        safe_generic: Some(safe_generic),
    } = uninit
    {
        record.generic(&safe_generic.full);
    }

    for (index, datum) in data.iter().enumerate() {
        match (&uninit, datum.allow_uninit()) {
            (_, false) | (UninitKind::False, true) => {
                record.field(&format!("pub {}", datum.name()), datum.type_name());
            }
            (UninitKind::Safe { .. }, true) => {
                record.field(
                    &format!("pub {}", datum.name()),
                    format!("std::marker::PhantomData<T{}>", index),
                );
            }
            (UninitKind::Unsafe, true) => {}
        }
    }

    if let Some(doc) = record_info.doc {
        record.doc(doc);
    }

    if let UninitKind::Safe {
        unsafe_record_name,
        safe_generic,
    } = uninit
    {
        let from_impl = scope.new_impl(record_info.name);
        if let Some(generic) = &safe_generic {
            from_impl
                .generic(&generic.full)
                .target_generic(&generic.short);
        }
        from_impl.impl_trait(format!("From<{}>", unsafe_record_name));

        let uninit_has_data = data.iter().any(|datum| !datum.allow_uninit());

        let from_fn = from_impl
            .new_fn("from")
            .arg(
                if uninit_has_data { "from" } else { "_from" },
                unsafe_record_name,
            )
            .ret("Self");
        from_fn.line(format!(
            "Self {{ {} }}",
            data.iter()
                .map(|datum| if !datum.allow_uninit() {
                    format!("{}: from.{}", datum.name(), datum.name())
                } else {
                    format!("{}: std::marker::PhantomData", datum.name())
                })
                .join(", ")
        ));
    }
}

fn generate_data_out_record(
    record_info: RecordInfo,
    inside_record_name: &str,
    data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let record = scope
        .new_struct(record_info.name)
        .vis("pub")
        .generic(CAP_GENERIC);

    let mut inside_record_type = Type::new(inside_record_name);
    inside_record_type.generic(CAP);
    record.field("pub record", inside_record_type);

    for datum in data {
        record.field(&format!("pub {}", datum.name()), datum.type_name());
    }

    if let Some(doc) = record_info.doc {
        record.doc(doc);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use fragment::FragmentGenerator;
    use pretty_assertions::assert_eq;
    use rand::Rng;
    use rand_chacha::rand_core::SeedableRng;
    use syn::File;

    use super::*;
    use crate::{
        generator::fragment::{clone::CloneImplGenerator, serde::SerdeImplGenerator},
        record::{
            definition::{DatumDefinitionOverride, RecordDefinitionBuilder},
            type_resolver::{StaticTypeResolver, TypeResolver},
        },
    };

    pub(crate) fn assert_fragment_eq(left: &str, right: &str) {
        let parsed_left = syn::parse_str::<File>(left).expect("left");
        let parsed_right = syn::parse_str::<File>(right).expect("right");
        if parsed_left != parsed_right {
            assert_eq!(left, right);
        }
    }

    fn add_one<R: TypeResolver>(
        definition: &mut RecordDefinitionBuilder<R>,
        rng: &mut rand_chacha::ChaCha8Rng,
        i: usize,
    ) {
        match rng.gen_range(0..7) {
            0 => {
                definition.add_datum_allow_uninit::<u8, _>(format!("field_{}", i));
            }
            1 => {
                definition.add_datum_allow_uninit::<u16, _>(format!("field_{}", i));
            }
            2 => {
                definition.add_datum_allow_uninit::<u32, _>(format!("field_{}", i));
            }
            3 => {
                definition.add_datum_allow_uninit::<u64, _>(format!("field_{}", i));
            }
            4 => {
                definition.add_datum::<String, _>(format!("field_{}", i));
            }
            5 => {
                definition.add_dynamic_datum(format!("field_{}", i), "Box<str>");
            }
            6 => {
                definition.add_datum_override::<Vec<()>, _>(
                    format!("field_{}", i),
                    DatumDefinitionOverride {
                        type_name: Some("Vec<usize>".to_owned()),
                        size: None,
                        align: None,
                        allow_uninit: None,
                    },
                );
            }
            i => unreachable!("Unhandled value {}", i),
        };
    }

    #[test]
    fn generators_with_random_definitions() {
        let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
        println!("Seed: {:#04x?}", rng.get_seed());

        let type_resolver = {
            let mut resolver = StaticTypeResolver::default();
            resolver.add_all_types();
            resolver
        };

        const MAX_DATA: usize = 32;
        for _ in 0..256 {
            let mut definition = RecordDefinitionBuilder::new(&type_resolver);
            let num_data = rng.gen_range(0..=MAX_DATA);
            for i in 0..num_data {
                add_one(&mut definition, &mut rng, i);
            }
            definition.close_record_variant();
            let mut removed = BTreeSet::new();
            for _ in 0..(num_data / 5) {
                let index = rng.gen_range(0..definition.data().len());
                if !removed.contains(&index) {
                    removed.insert(index);
                    definition.remove_datum(definition.data()[index].id());
                }
            }
            for i in 0..(num_data / 5) {
                add_one(&mut definition, &mut rng, num_data + i);
            }
            let def = definition.build();
            generate(
                &def,
                &GeneratorConfig::default_with_custom_generators([
                    Box::new(CloneImplGenerator) as Box<dyn FragmentGenerator>,
                    Box::new(SerdeImplGenerator) as Box<dyn FragmentGenerator>,
                ]),
            );
        }
    }
}
