use crate::record::definition::{DatumDefinition, RecordDefinition, RecordVariant};
use codegen::{Impl, Scope, Type};
use itertools::{Either, EitherOrBoth, Itertools};
use std::{collections::BTreeSet, ops::Deref};

const CAP_GENERIC: &str = "const CAP: usize";
const CAP: &str = "CAP";

pub fn generate(definition: &RecordDefinition) -> String {
    let mut scope = Scope::new();

    scope.import("truc_runtime::data", "RecordMaybeUninit");

    let mut uninit_type = Type::new("RecordMaybeUninit");
    uninit_type.generic(CAP);

    let max_size = definition
        .datum_definitions()
        .map(|d| d.offset() + d.size())
        .max()
        .unwrap_or(0);

    scope.raw(&format!(
        r#"/// Maximum size of the record, regardless of the record variant.
///
/// Use that value, or a greater value, as the `CAP` const generic of every record.
pub const MAX_SIZE: usize = {};"#,
        max_size
    ));

    let record_uninit = scope
        .new_struct("RecordUninitialized")
        .vis("pub")
        .generic(CAP_GENERIC);
    record_uninit.field("_data", &uninit_type);
    record_uninit.doc(
        r#"Uninitialized record.

It will never drop any data except the container by itself.

# Intention

This is to be used in custom allocators."#,
    );

    let mut prev_variant = None::<(&RecordVariant, String)>;

    let mut type_size_assertions = BTreeSet::new();

    for variant in definition.variants() {
        let data = variant
            .data()
            .sorted()
            .map(|d| {
                definition
                    .get_datum_definition(d)
                    .unwrap_or_else(|| panic!("datum #{}", d))
            })
            .collect::<Vec<_>>();

        let capped_record_name = format!("CappedRecord{}", variant.id());
        let record_name = format!("Record{}", variant.id());
        let unpacked_record_name = format!("UnpackedRecord{}", variant.id());
        let unpacked_uninit_record_name = format!("UnpackedUninitRecord{}", variant.id());
        let unpacked_uninit_safe_record_name = format!("UnpackedUninitSafeRecord{}", variant.id());

        let (minus_data, plus_data) =
            if let Some((prev_variant, _prev_capped_record_name)) = &prev_variant {
                prev_variant
                    .data()
                    .sorted()
                    .merge_join_by(&data, |left_id, right| left_id.cmp(&right.id()))
                    .filter_map(|either| match either {
                        EitherOrBoth::Left(left_id) => Some(Either::Left(
                            definition
                                .get_datum_definition(left_id)
                                .unwrap_or_else(|| panic!("datum #{}", left_id)),
                        )),
                        EitherOrBoth::Right(right) => Some(Either::Right(right)),
                        EitherOrBoth::Both(_, _) => None,
                    })
                    .partition_map::<Vec<_>, Vec<_>, _, _, _>(|e| e)
            } else {
                (Vec::new(), data.clone())
            };
        for datum in &plus_data {
            type_size_assertions.insert((datum.type_name(), datum.size()));
        }

        generate_data_record(
            RecordInfo {
                name: &unpacked_record_name,
                public: true,
                doc: Some(
                    r#"Data container for packing/unpacking records.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                ),
            },
            &data,
            UninitKind::False,
            &mut scope,
        );
        generate_data_record(
            RecordInfo {
                name: &unpacked_uninit_record_name,
                public: true,
                doc: Some(
                    r#"Data container for packing/unpacking records without the data to be left uninitialized.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                ),
            },
            &data,
            UninitKind::Unsafe,
            &mut scope,
        );
        let unpacked_uninit_record_generic = generate_data_record(
            RecordInfo {
                name: &unpacked_uninit_safe_record_name,
                public: false,
                doc: Some(
                    r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                ),
            },
            &data,
            UninitKind::Safe {
                unsafe_record_name: &unpacked_uninit_record_name,
            },
            &mut scope,
        );
        let unpacked_uninit_info = UninitInfo {
            record_name: &unpacked_uninit_record_name,
            safe_record_name: &unpacked_uninit_safe_record_name,
            record_generic: &unpacked_uninit_record_generic,
        };

        let record = scope
            .new_struct(&capped_record_name)
            .vis("pub")
            .generic(CAP_GENERIC);
        record.field("data", &uninit_type);
        if let Some((prev_variant, _)) = prev_variant {
            record.doc(&format!(
                r#"Record variant #{}.

It may be converted from a [`Record{}`] via one of the various call to [`From::from`]

It may also be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                variant.id(),
                prev_variant.id()
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
            record_name,
            capped_record_name,
        ));

        generate_record_impl(
            RecordImplRecordNames {
                name: &capped_record_name,
                unpacked: &unpacked_record_name,
            },
            &unpacked_uninit_info,
            &data,
            &mut scope,
        );
        generate_drop_impl(&capped_record_name, &data, &mut scope);
        generate_from_constructor_record_impl(
            RecordImplRecordNames {
                name: &capped_record_name,
                unpacked: &unpacked_record_name,
            },
            false,
            &mut scope,
        );
        generate_from_constructor_record_impl(
            RecordImplRecordNames {
                name: &capped_record_name,
                unpacked: &unpacked_uninit_record_name,
            },
            true,
            &mut scope,
        );
        if let Some((prev_variant, prev_capped_record_name)) = &prev_variant {
            let plus_record_name = format!("UnpackedRecordIn{}", variant.id());
            let uninit_plus_record_name = format!("UnpackedUninitRecordIn{}", variant.id());
            let uninit_safe_plus_record_name =
                format!("UnpackedUninitSafeRecordIn{}", variant.id());
            let and_out_record_name = format!("Record{}AndUnpackedOut", variant.id());

            generate_data_record(
                RecordInfo {
                    name: &plus_record_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`]."#,
                        prev_variant.id()
                    )),
                },
                &plus_data,
                UninitKind::False,
                &mut scope,
            );
            generate_data_record(
                RecordInfo {
                    name: &uninit_plus_record_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`] without the data to be left uninitialized."#,
                        prev_variant.id()
                    )),
                },
                &plus_data,
                UninitKind::Unsafe,
                &mut scope,
            );
            let uninit_plus_record_generic = generate_data_record(
                RecordInfo {
                    name: &uninit_safe_plus_record_name,
                    public: false,
                    doc: Some(
                        r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                    ),
                },
                &plus_data,
                UninitKind::Safe {
                    unsafe_record_name: &uninit_plus_record_name,
                },
                &mut scope,
            );
            let plus_uninit_info = UninitInfo {
                record_name: &uninit_plus_record_name,
                safe_record_name: &uninit_safe_plus_record_name,
                record_generic: &uninit_plus_record_generic,
            };

            generate_data_out_record(
                RecordInfo {
                    name: &and_out_record_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Result of conversion from record variant #{} to variant #{} via a [`From::from`] call.

It contains all the removed data so that one can still use them, or drop them."#,
                        prev_variant.id(),
                        variant.id()
                    )),
                },
                &capped_record_name,
                &minus_data,
                &mut scope,
            );

            generate_from_previous_record_impl(
                RecordFromPreviousRecordNames {
                    name: &capped_record_name,
                    prev: prev_capped_record_name,
                    plus: &plus_record_name,
                    and_out: None,
                },
                None,
                &minus_data,
                &plus_data,
                &mut scope,
            );
            generate_from_previous_record_impl(
                RecordFromPreviousRecordNames {
                    name: &capped_record_name,
                    prev: prev_capped_record_name,
                    plus: &uninit_plus_record_name,
                    and_out: None,
                },
                Some(&plus_uninit_info),
                &minus_data,
                &plus_data,
                &mut scope,
            );

            generate_from_previous_record_impl(
                RecordFromPreviousRecordNames {
                    name: &capped_record_name,
                    prev: prev_capped_record_name,
                    plus: &plus_record_name,
                    and_out: Some(&and_out_record_name),
                },
                None,
                &minus_data,
                &plus_data,
                &mut scope,
            );

            generate_from_previous_record_impl(
                RecordFromPreviousRecordNames {
                    name: &capped_record_name,
                    prev: prev_capped_record_name,
                    plus: &uninit_plus_record_name,
                    and_out: Some(&and_out_record_name),
                },
                Some(&plus_uninit_info),
                &minus_data,
                &plus_data,
                &mut scope,
            );
        }

        prev_variant = Some((variant, capped_record_name));
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

fn generate_record_impl(
    record_names: RecordImplRecordNames,
    uninit_info: &UninitInfo,
    data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let record_impl = scope
        .new_impl(record_names.name)
        .generic(CAP_GENERIC)
        .target_generic(CAP);

    generate_constructor(data, record_names.unpacked, None, record_impl);
    generate_constructor(
        data,
        uninit_info.record_name,
        Some(uninit_info),
        record_impl,
    );

    generate_unpacker(data, record_names.unpacked, record_impl);

    for datum in data {
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

struct UninitInfo<'a> {
    record_name: &'a str,
    safe_record_name: &'a str,
    record_generic: &'a Option<RecordGeneric>,
}

fn generate_constructor(
    data: &[&DatumDefinition],
    unpacked_record_name: &str,
    uninit: Option<&UninitInfo>,
    record_impl: &mut Impl,
) {
    let (has_data, uninit_has_data) = {
        (
            !data.is_empty(),
            if uninit.is_some() {
                data.iter().any(|datum| !datum.allow_uninit())
            } else {
                false
            },
        )
    };
    let new_fn = record_impl
        .new_fn(if uninit.is_none() {
            "new"
        } else {
            "new_uninit"
        })
        .vis("pub")
        .arg(
            if uninit.is_some() || has_data {
                "from"
            } else {
                "_from"
            },
            unpacked_record_name,
        )
        .ret("Self");
    if let Some(uninit) = uninit {
        new_fn.line(format!(
            "let {} = {}{}::from(from);",
            if uninit_has_data { "from" } else { "_from" },
            uninit.safe_record_name,
            uninit
                .record_generic
                .as_ref()
                .map_or_else(String::new, |generic| format!("::<{}>", generic.typed))
        ));
    }
    new_fn.line(format!(
        "let {}data = RecordMaybeUninit::new();",
        if uninit.is_none() && has_data || uninit.is_some() && uninit_has_data {
            "mut "
        } else {
            ""
        }
    ));
    for datum in data
        .iter()
        .filter(|datum| uninit.is_none() || !datum.allow_uninit())
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

fn generate_drop_impl(record_name: &str, data: &[&DatumDefinition], scope: &mut Scope) {
    let drop_impl = scope
        .new_impl(record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait("Drop");

    let drop_fn = drop_impl.new_fn("drop").arg_mut_self();
    for datum in data {
        drop_fn.line(format!(
            "let _{}: {} = unsafe {{ self.data.read({}) }};",
            datum.name(),
            datum.type_name(),
            datum.offset(),
        ));
    }
}

fn generate_from_constructor_record_impl(
    record_names: RecordImplRecordNames,
    uninit: bool,
    scope: &mut Scope,
) {
    let from_impl = scope
        .new_impl(record_names.name)
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait(format!("From<{}>", record_names.unpacked));

    let from_fn = from_impl
        .new_fn("from")
        .arg("from", record_names.unpacked)
        .ret("Self");
    from_fn.line(format!(
        "Self::{}(from)",
        if !uninit { "new" } else { "new_uninit" },
    ));
}

struct RecordFromPreviousRecordNames<'a> {
    name: &'a str,
    prev: &'a str,
    plus: &'a str,
    and_out: Option<&'a str>,
}

fn generate_from_previous_record_impl(
    record_names: RecordFromPreviousRecordNames,
    uninit: Option<&UninitInfo>,
    minus_data: &[&DatumDefinition],
    plus_data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let from_type = format!("({}<{}>, {})", record_names.prev, CAP, record_names.plus);
    let from_impl = scope
        .new_impl(record_names.and_out.unwrap_or(record_names.name))
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait(format!("From<{}>", from_type));

    let (plus_has_data, uninit_plus_has_data) = {
        (
            !plus_data.is_empty(),
            if uninit.is_some() {
                plus_data.iter().any(|datum| !datum.allow_uninit())
            } else {
                false
            },
        )
    };

    let from_fn = from_impl
        .new_fn("from")
        .arg(
            if uninit.is_some() || plus_has_data {
                "(from, plus)"
            } else {
                "(from, _plus)"
            },
            from_type,
        )
        .ret("Self");

    for datum in minus_data {
        from_fn.line(format!(
            "let {}{}: {} = unsafe {{ from.data.read({}) }};",
            if record_names.and_out.is_some() {
                ""
            } else {
                "_"
            },
            datum.name(),
            datum.type_name(),
            datum.offset(),
        ));
    }

    if let Some(uninit) = uninit {
        from_fn.line(format!(
            "let {} = {}{}::from(plus);",
            if uninit_plus_has_data {
                "plus"
            } else {
                "_plus"
            },
            uninit.safe_record_name,
            uninit
                .record_generic
                .as_ref()
                .map_or_else(String::new, |generic| format!("::<{}>", generic.typed))
        ));
    }
    from_fn.line("let manually_drop = std::mem::ManuallyDrop::new(from);");
    from_fn.line(format!(
        "let {}data = unsafe {{ std::ptr::read(&(*manually_drop).data) }};",
        if uninit.is_none() && plus_has_data || uninit.is_some() && uninit_plus_has_data {
            "mut "
        } else {
            ""
        }
    ));

    for datum in plus_data
        .iter()
        .filter(|datum| uninit.is_none() || !datum.allow_uninit())
    {
        from_fn.line(format!(
            "unsafe {{ data.write({}, plus.{}); }}",
            datum.offset(),
            datum.name(),
        ));
    }
    if let Some(and_out_record_name) = record_names.and_out {
        from_fn.line(format!("let record = {} {{ data }};", record_names.name));
        from_fn.line(format!(
            "{} {{ record{} }}",
            and_out_record_name,
            minus_data
                .iter()
                .flat_map(|datum| [", ", datum.name()])
                .collect::<String>()
        ));
    } else {
        from_fn.line("Self { data }");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UninitKind<'a> {
    False,
    Unsafe,
    Safe { unsafe_record_name: &'a str },
}

struct RecordGeneric {
    full: String,
    short: String,
    typed: String,
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
) -> Option<RecordGeneric> {
    let record = scope.new_struct(record_info.name);
    if record_info.public {
        record.vis("pub");
    }

    let (generic, uninit_has_data) = match &uninit {
        UninitKind::Safe { .. } => {
            let mut uninit_has_data = false;
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
                } else {
                    uninit_has_data = true;
                }
            }

            (
                if !generic.is_empty() {
                    record.generic(&generic);

                    Some(RecordGeneric {
                        full: generic,
                        short: short_generic,
                        typed: typed_generic,
                    })
                } else {
                    None
                },
                uninit_has_data,
            )
        }
        UninitKind::False | UninitKind::Unsafe => (None, false),
    };

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

    if let UninitKind::Safe { unsafe_record_name } = uninit {
        let from_impl = scope.new_impl(record_info.name);
        if let Some(generic) = &generic {
            from_impl
                .generic(&generic.full)
                .target_generic(&generic.short);
        }
        from_impl.impl_trait(format!("From<{}>", unsafe_record_name));

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

    generic
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
