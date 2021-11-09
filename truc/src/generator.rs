use crate::record::definition::{DatumDefinition, RecordDefinition};
use codegen::{Impl, Scope, Type};
use itertools::Itertools;
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

fn generate_data_record(record_name: &str, data: &[&DatumDefinition], scope: &mut Scope) {
    let record = scope.new_struct(record_name).vis("pub");

    for datum in data {
        record.field(&format!("pub {}", datum.name()), datum.type_name());
    }
}
