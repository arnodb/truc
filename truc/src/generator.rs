use crate::record::definition::RecordDefinition;
use codegen::{Scope, Type};
use std::io::Write;

pub fn generate<W: Write>(
    definition: &RecordDefinition,
    output: &mut W,
) -> Result<(), std::io::Error> {
    let mut scope = Scope::new();

    scope.import("truc_runtime::data", "RecordMaybeUninit");

    const CAP_GENERIC: &str = "const CAP: usize";
    const CAP: &str = "CAP";

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
        let record_name = format!("Record{}", variant.id());

        let data = variant
            .data()
            .map(|d| definition.get_datum_definition(d).expect("datum"))
            .collect::<Vec<_>>();

        let record = scope
            .new_struct(&record_name)
            .vis("pub")
            .generic(CAP_GENERIC);
        record.field("data", &uninit_type);

        let record_impl = scope
            .new_impl(&record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP);

        let new_fn = record_impl.new_fn("new").vis("pub").ret("Self");
        for datum in &data {
            new_fn.arg(&format!("datum_{}", datum.id()), datum.type_name());
        }
        new_fn.line("let mut data = RecordMaybeUninit::new();");
        for datum in &data {
            new_fn.line(format!(
                "unsafe {{ data.write({}, datum_{}); }}",
                datum.offset(),
                datum.id()
            ));
        }
        new_fn.line("Self { data }");

        let drop_impl = scope
            .new_impl(&record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait("Drop");

        let drop_fn = drop_impl.new_fn("drop").arg_mut_self();
        for datum in &data {
            drop_fn.line(format!(
                "let _datum{}: {} = unsafe {{ self.data.read({}) }};",
                datum.id(),
                datum.type_name(),
                datum.offset(),
            ));
        }
    }

    write!(output, "{}", scope.to_string())?;
    Ok(())
}
