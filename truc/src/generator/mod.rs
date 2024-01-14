use std::collections::BTreeSet;

use codegen::{Scope, Type};
use itertools::{Either, EitherOrBoth, Itertools};

use self::{
    config::GeneratorConfig,
    fragment::{
        drop_impl::DropImplGenerator,
        from_previous_record_data_records::FromPreviousRecordDataRecordsGenerator,
        from_previous_record_impls::FromPreviousRecordImplsGenerator,
        from_unpacked_record_impls::FromUnpackedRecordImplsGenerator,
        record_impl::RecordImplGenerator, FragmentGenerator, FragmentGeneratorSpecs, RecordGeneric,
        RecordSpec,
    },
};
use crate::record::definition::{DatumDefinition, RecordDefinition};

pub mod config;
pub mod fragment;

const CAP_GENERIC: &str = "const CAP: usize";
const CAP: &str = "CAP";

pub fn generate(definition: &RecordDefinition, config: &GeneratorConfig) -> String {
    let mut scope = Scope::new();

    scope.import("truc_runtime::data", "RecordMaybeUninit");

    for customizer in &config.custom_fragment_generators {
        customizer.imports(&mut scope);
    }

    let mut uninit_type = Type::new("RecordMaybeUninit");
    uninit_type.generic(CAP);

    let max_type_align = definition.max_type_align();
    let max_size = definition.max_size();

    scope.raw(&format!(
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
            variant,
            capped_record_name: format!("CappedRecord{}", variant.id()),
            record_name: format!("Record{}", variant.id()),
            unpacked_record_name: format!("UnpackedRecord{}", variant.id()),
            unpacked_uninit_record_name: format!("UnpackedUninitRecord{}", variant.id()),
            unpacked_uninit_safe_record_name: format!("UnpackedUninitSafeRecord{}", variant.id()),
            unpacked_record_in_name: format!("UnpackedRecordIn{}", variant.id()),
            unpacked_uninit_record_in_name: format!("UnpackedUninitRecordIn{}", variant.id()),
            unpacked_uninit_safe_record_in_name: format!(
                "UnpackedUninitSafeRecordIn{}",
                variant.id()
            ),
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

        generate_data_record(
            RecordInfo {
                name: &record_spec.unpacked_record_name,
                public: true,
                doc: Some(
                    r#"Data container for packing/unpacking records.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                ),
            },
            &record_spec.data,
            UninitKind::False,
            &mut scope,
        );
        generate_data_record(
            RecordInfo {
                name: &record_spec.unpacked_uninit_record_name,
                public: true,
                doc: Some(
                    r#"Data container for packing/unpacking records without the data to be left uninitialized.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                ),
            },
            &record_spec.data,
            UninitKind::Unsafe,
            &mut scope,
        );
        generate_data_record(
            RecordInfo {
                name: &record_spec.unpacked_uninit_safe_record_name,
                public: false,
                doc: Some(
                    r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                ),
            },
            &record_spec.data,
            UninitKind::Safe {
                unsafe_record_name: &record_spec.unpacked_uninit_record_name,
                safe_generic: record_spec.unpacked_uninit_safe_generic.as_ref(),
            },
            &mut scope,
        );

        let record = scope
            .new_struct(&record_spec.capped_record_name)
            .repr(&format!("align({})", max_type_align))
            .vis("pub")
            .generic(CAP_GENERIC);
        record.field("data", &uninit_type);
        if let Some(prev_record_spec) = prev_record_spec.as_ref() {
            record.doc(&format!(
                r#"Record variant #{}.

It may be converted from a [`Record{}`] via one of the various call to [`From::from`]

It may also be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                variant.id(),
                prev_record_spec.variant.id()
            ));
        } else {
            record.doc(&format!(
                r#"Record variant #{}.

It may be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                variant.id()
            ));
        }

        scope.raw(&format!(
            r#"/// Record variant #{} with optimized capacity.
pub type {} = {}<{{ MAX_SIZE }}>;"#,
            variant.id(),
            record_spec.record_name,
            record_spec.capped_record_name,
        ));

        let specs = FragmentGeneratorSpecs {
            record: &record_spec,
            prev_record: prev_record_spec,
        };

        let common_fragment_generators: [Box<dyn FragmentGenerator>; 5] = [
            Box::new(RecordImplGenerator),
            Box::new(DropImplGenerator),
            Box::new(FromUnpackedRecordImplsGenerator),
            Box::new(FromPreviousRecordDataRecordsGenerator),
            Box::new(FromPreviousRecordImplsGenerator),
        ];
        let fragment_generators = common_fragment_generators
            .iter()
            .chain(config.custom_fragment_generators.iter());

        for generator in fragment_generators {
            generator.generate(&specs, &mut scope);
        }

        prev_record_spec = Some(record_spec);
    }

    // This checks there is no type substitution which could lead to unsafe
    // code due to different type size.
    for (type_name, size) in type_size_assertions {
        scope.raw(&format!(
            "const_assert_eq!(std::mem::size_of::<{}>(), {});",
            type_name, size
        ));
    }

    scope.to_string()
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

struct RecordInfo<'a> {
    name: &'a str,
    public: bool,
    doc: Option<&'a str>,
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
