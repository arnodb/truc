use std::{env, fs::File, io::Write, path::PathBuf};

use truc::{
    generator::{config::GeneratorConfig, generate},
    record::{definition::RecordDefinitionBuilder, type_resolver::HostTypeResolver},
};

fn main() {
    let mut definition = RecordDefinitionBuilder::new(&HostTypeResolver);

    // First variant with an integer
    let integer_id = definition.add_datum_allow_uninit::<usize, _>("integer");
    definition.close_record_variant();

    // Second variant with a string
    let string_id = definition.add_datum::<String, _>("string");
    definition.remove_datum(integer_id);
    definition.close_record_variant();

    // Remove the integer and replace it with another
    definition.add_datum_allow_uninit::<isize, _>("signed_integer");
    definition.remove_datum(string_id);
    definition.close_record_variant();

    // Build
    let definition = definition.build();

    // Generate Rust definitions
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let out_dir_path = PathBuf::from(out_dir);
    let mut file = File::create(out_dir_path.join("readme_truc.rs")).unwrap();
    write!(
        file,
        "{}",
        generate(&definition, &GeneratorConfig::default())
    )
    .unwrap();
}
