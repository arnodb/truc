use crate::record::definition::{DatumDefinition, RecordDefinition, RecordVariant};
use codegen::{Impl, Scope, Type};
use itertools::{Either, EitherOrBoth, Itertools};
use std::io::Write;

const CAP_GENERIC: &str = "const CAP: usize";
const CAP: &str = "CAP";

pub fn generate<W: Write>(
    definition: &RecordDefinition,
    output: &mut W,
) -> Result<(), std::io::Error> {
    let mut scope = Scope::new();

    scope.import("truc_runtime::data", "RecordMaybeUninit");

    let mut uninit_type = Type::new("RecordMaybeUninit");
    uninit_type.generic(CAP);

    let max_size = definition
        .datum_definitions()
        .map(|d| d.offset() + d.size())
        .max()
        .unwrap_or(0);

    scope.raw(&format!("pub const MAX_SIZE: usize = {};", max_size));

    let record_uninit = scope
        .new_struct("RecordUninitialized")
        .vis("pub")
        .generic(CAP_GENERIC);
    record_uninit.field("_data", &uninit_type);

    let mut prev_variant = None::<(&RecordVariant, String)>;

    for variant in definition.variants() {
        let data = variant
            .data()
            .sorted()
            .map(|d| definition.get_datum_definition(d).expect("datum"))
            .collect::<Vec<_>>();

        let record_name = format!("Record{}", variant.id());
        let constructor_record_name = format!("NewRecord{}", variant.id());
        let uninit_constructor_record_name = format!("NewRecordUninit{}", variant.id());
        let uninit_safe_constructor_record_name = format!("NewRecordUninitSafe{}", variant.id());

        generate_data_record(
            &constructor_record_name,
            &data,
            UninitKind::False,
            &mut scope,
        );
        generate_data_record(
            &uninit_constructor_record_name,
            &data,
            UninitKind::Unsafe,
            &mut scope,
        );
        let uninit_constructor_record_generic = generate_data_record(
            &uninit_safe_constructor_record_name,
            &data,
            UninitKind::Safe {
                unsafe_record_name: uninit_constructor_record_name.clone(),
            },
            &mut scope,
        );

        let record = scope
            .new_struct(&record_name)
            .vis("pub")
            .generic(CAP_GENERIC);
        record.field("data", &uninit_type);

        generate_record_impl(
            &data,
            &record_name,
            &constructor_record_name,
            &uninit_safe_constructor_record_name,
            &uninit_constructor_record_generic,
            &mut scope,
        );
        generate_drop_impl(&record_name, &data, &mut scope);
        generate_from_constructor_record_impl(
            &record_name,
            &constructor_record_name,
            false,
            &uninit_safe_constructor_record_name,
            &mut scope,
        );
        generate_from_constructor_record_impl(
            &record_name,
            &uninit_constructor_record_name,
            true,
            &uninit_safe_constructor_record_name,
            &mut scope,
        );
        if let Some((prev_variant, prev_record_name)) = prev_variant {
            let (minus_data, plus_data) = prev_variant
                .data()
                .sorted()
                .merge_join_by(data, |left_id, right| left_id.cmp(&right.id()))
                .filter_map(|either| match either {
                    EitherOrBoth::Left(left_id) => Some(Either::Left(
                        definition.get_datum_definition(left_id).expect("datum"),
                    )),
                    EitherOrBoth::Right(right) => Some(Either::Right(right)),
                    EitherOrBoth::Both(_, _) => None,
                })
                .partition_map::<Vec<_>, Vec<_>, _, _, _>(|e| e);

            let plus_record_name = format!("RecordIn{}", variant.id());
            let and_out_record_name = format!("Record{}AndOut", variant.id());

            generate_data_record(&plus_record_name, &plus_data, UninitKind::False, &mut scope);
            generate_data_out_record(&and_out_record_name, &record_name, &minus_data, &mut scope);

            generate_from_previous_record_impl(
                &record_name,
                &prev_record_name,
                &plus_record_name,
                &minus_data,
                &plus_data,
                &mut scope,
            );

            generate_from_previous_record_minus_impl(
                &record_name,
                &prev_record_name,
                &plus_record_name,
                &minus_data,
                &plus_data,
                &and_out_record_name,
                &mut scope,
            );
        }

        prev_variant = Some((variant, record_name));
    }

    write!(output, "{}", scope.to_string())?;
    Ok(())
}

fn generate_record_impl(
    data: &[&DatumDefinition],
    record_name: &str,
    constructor_record_name: &str,
    uninit_safe_constructor_record_name: &str,
    uninit_constructor_record_generic: &Option<RecordGeneric>,
    scope: &mut Scope,
) {
    let record_impl = scope
        .new_impl(record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP);

    generate_constructor(data, constructor_record_name, false, &None, record_impl);
    generate_constructor(
        data,
        uninit_safe_constructor_record_name,
        true,
        uninit_constructor_record_generic,
        record_impl,
    );

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

fn generate_constructor(
    data: &[&DatumDefinition],
    constructor_record_name: &str,
    uninit: bool,
    uninit_constructor_record_generic: &Option<RecordGeneric>,
    record_impl: &mut Impl,
) {
    let mut filtered_data = data
        .iter()
        .filter(|datum| !uninit || !datum.allow_uninit())
        .peekable();
    let unused_from = filtered_data.peek().is_none();
    let from_type = match (uninit, uninit_constructor_record_generic) {
        (true, Some(generic)) => {
            format!("{}<{}>", constructor_record_name, generic.typed)
        }
        (true, None) | (false, _) => constructor_record_name.to_string(),
    };
    let new_fn = record_impl
        .new_fn(if !uninit { "new" } else { "new_uninit" })
        .vis("pub")
        .arg(if !unused_from { "from" } else { "_from" }, from_type)
        .ret("Self");
    new_fn.line(format!(
        "let {} data = RecordMaybeUninit::new();",
        if !unused_from { "mut" } else { "" }
    ));
    for datum in filtered_data {
        new_fn.line(format!(
            "unsafe {{ data.write({}, from.{}); }}",
            datum.offset(),
            datum.name()
        ));
    }
    new_fn.line("Self { data }");
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
    record_name: &str,
    constructor_record_name: &str,
    uninit: bool,
    uninit_safe_constructor_record_name: &str,
    scope: &mut Scope,
) {
    let from_impl = scope
        .new_impl(record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait(format!("From<{}>", constructor_record_name));

    let from_fn = from_impl
        .new_fn("from")
        .arg("from", constructor_record_name)
        .ret("Self");
    if !uninit {
        from_fn.line(format!(
            "Self::{}(from)",
            if !uninit { "new" } else { "new_uninit" },
        ));
    } else {
        from_fn.line(format!(
            "Self::{}({}::from(from))",
            if !uninit { "new" } else { "new_uninit" },
            uninit_safe_constructor_record_name
        ));
    }
}

fn generate_from_previous_record_impl(
    record_name: &str,
    prev_record_name: &str,
    plus_record_name: &str,
    minus_data: &[&DatumDefinition],
    plus_data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let from_type = format!("({}<{}>, {})", prev_record_name, CAP, plus_record_name);
    let from_impl = scope
        .new_impl(record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait(format!("From<{}>", from_type));

    let from_fn = from_impl
        .new_fn("from")
        .arg(
            &format!(
                "(from, {}plus)",
                if plus_data.is_empty() { "_" } else { "" }
            ),
            from_type,
        )
        .ret("Self");

    for datum in minus_data {
        from_fn.line(format!(
            "let _{}: {} = unsafe {{ from.data.read({}) }};",
            datum.name(),
            datum.type_name(),
            datum.offset(),
        ));
    }

    from_fn.line("let manually_drop = std::mem::ManuallyDrop::new(from);");
    from_fn.line(format!(
        "let {}data = unsafe {{ std::ptr::read(&(*manually_drop).data) }};",
        if plus_data.is_empty() { "" } else { "mut " }
    ));

    for datum in plus_data {
        from_fn.line(format!(
            "unsafe {{ data.write({}, plus.{}); }}",
            datum.offset(),
            datum.name(),
        ));
    }
    from_fn.line("Self { data }");
}

fn generate_from_previous_record_minus_impl(
    record_name: &str,
    prev_record_name: &str,
    plus_record_name: &str,
    minus_data: &[&DatumDefinition],
    plus_data: &[&DatumDefinition],
    and_out_record_name: &str,
    scope: &mut Scope,
) {
    let from_type = format!("({}<{}>, {})", prev_record_name, CAP, plus_record_name);
    let from_impl = scope
        .new_impl(and_out_record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP)
        .impl_trait(format!("From<{}>", from_type));

    let from_fn = from_impl
        .new_fn("from")
        .arg(
            &format!(
                "(from, {}plus)",
                if plus_data.is_empty() { "_" } else { "" }
            ),
            from_type,
        )
        .ret("Self");

    for datum in minus_data {
        from_fn.line(format!(
            "let {}: {} = unsafe {{ from.data.read({}) }};",
            datum.name(),
            datum.type_name(),
            datum.offset(),
        ));
    }

    from_fn.line("let manually_drop = std::mem::ManuallyDrop::new(from);");
    from_fn.line(format!(
        "let {}data = unsafe {{ std::ptr::read(&(*manually_drop).data) }};",
        if plus_data.is_empty() { "" } else { "mut " }
    ));

    for datum in plus_data {
        from_fn.line(format!(
            "unsafe {{ data.write({}, plus.{}); }}",
            datum.offset(),
            datum.name(),
        ));
    }
    from_fn.line(format!("let record = {} {{ data }};", record_name));
    from_fn.line(format!(
        "{} {{ record{} }}",
        and_out_record_name,
        minus_data
            .iter()
            .flat_map(|datum| [", ", datum.name()])
            .collect::<String>()
    ));
}

#[derive(Debug, PartialEq, Eq)]
enum UninitKind {
    False,
    Unsafe,
    Safe { unsafe_record_name: String },
}

struct RecordGeneric {
    full: String,
    short: String,
    typed: String,
}

fn generate_data_record(
    record_name: &str,
    data: &[&DatumDefinition],
    uninit: UninitKind,
    scope: &mut Scope,
) -> Option<RecordGeneric> {
    let record = scope.new_struct(record_name).vis("pub");

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

    if let UninitKind::Safe { unsafe_record_name } = &uninit {
        let from_impl = scope.new_impl(record_name);
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
    record_name: &str,
    inside_record_name: &str,
    data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let record = scope
        .new_struct(record_name)
        .vis("pub")
        .generic(CAP_GENERIC);

    let mut inside_record_type = Type::new(inside_record_name);
    inside_record_type.generic(CAP);
    record.field("pub record", inside_record_type);

    for datum in data {
        record.field(&format!("pub {}", datum.name()), datum.type_name());
    }
}
