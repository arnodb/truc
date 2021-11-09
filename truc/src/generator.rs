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

        generate_data_record(&constructor_record_name, &data, &mut scope);

        let record = scope
            .new_struct(&record_name)
            .vis("pub")
            .generic(CAP_GENERIC);
        record.field("data", &uninit_type);

        generate_record_impl(&data, &record_name, &constructor_record_name, &mut scope);
        generate_drop_impl(&record_name, &data, &mut scope);
        generate_from_constructor_record_impl(&record_name, &constructor_record_name, &mut scope);
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

            let plus_record_name = format!("ToRecord{}", variant.id());

            generate_data_record(&plus_record_name, &plus_data, &mut scope);

            generate_from_previous_record_impl(
                &record_name,
                &prev_record_name,
                &plus_record_name,
                &minus_data,
                &plus_data,
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
    scope: &mut Scope,
) {
    let record_impl = scope
        .new_impl(record_name)
        .generic(CAP_GENERIC)
        .target_generic(CAP);

    generate_constructor(data, constructor_record_name, record_impl);

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
    record_impl: &mut Impl,
) {
    let new_fn = record_impl
        .new_fn("new")
        .vis("pub")
        .arg("from", constructor_record_name)
        .ret("Self");
    new_fn.line("let mut data = RecordMaybeUninit::new();");
    for datum in data {
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
    from_fn.line("Self::new(from)");
}

fn generate_from_previous_record_impl(
    record_name: &str,
    prev_record_name: &str,
    plus_record_name: &str,
    minus_data: &[&DatumDefinition],
    plus_data: &[&DatumDefinition],
    scope: &mut Scope,
) {
    let from_type = format!("({}<CAP>, {})", prev_record_name, plus_record_name);
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

fn generate_data_record(record_name: &str, data: &[&DatumDefinition], scope: &mut Scope) {
    let record = scope.new_struct(record_name).vis("pub");

    for datum in data {
        record.field(&format!("pub {}", datum.name()), datum.type_name());
    }
}
