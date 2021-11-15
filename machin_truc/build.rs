use machin_data::MachinEnum;
use std::env;
use std::fs::File;
use std::path::Path;
use truc::generator::generate;
use truc::record::definition::RecordDefinitionBuilder;

fn machin() {
    let mut definition = RecordDefinitionBuilder::new();

    let a0 = definition.add_datum_allow_uninit::<u32, _>("datum_a");
    let b0 = definition.add_datum_allow_uninit::<u32, _>("datum_b");
    definition.close_record_variant();

    let c1 = definition.add_datum_allow_uninit::<u32, _>("datum_c");
    definition.close_record_variant();

    definition.remove_datum(a0);
    definition.close_record_variant();

    let d3 = definition.add_datum_allow_uninit::<u8, _>("datum_d");
    let e3 = definition.add_datum_allow_uninit::<u16, _>("datum_e");
    let f3 = definition.add_datum_allow_uninit::<u32, _>("datum_f");
    definition.close_record_variant();

    let machin_enum = definition.add_datum::<MachinEnum, _>("machin_enum");
    definition.close_record_variant();

    definition.remove_datum(b0);
    definition.remove_datum(c1);
    definition.remove_datum(d3);
    definition.remove_datum(e3);
    definition.remove_datum(f3);
    definition.remove_datum(machin_enum);
    let _datum_string = definition.add_datum::<String, _>("datum_string");
    let _datum_array_of_strings = definition.add_datum::<[String; 2], _>("datum_array_of_strings");

    let definition = definition.build();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);
    let mut file = File::create(&dest.join("machin_truc.rs")).unwrap();
    generate(&definition, &mut file).unwrap();
}

fn index_first_char() {
    let mut definition = RecordDefinitionBuilder::new();

    let words = definition.add_datum::<String, _>("words");
    definition.close_record_variant();

    definition.add_datum::<String, _>("word");
    definition.remove_datum(words);
    definition.close_record_variant();

    definition.add_datum::<Option<char>, _>("first_char");
    definition.close_record_variant();

    let definition = definition.build();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);
    let mut file = File::create(&dest.join("index_first_char.rs")).unwrap();
    generate(&definition, &mut file).unwrap();
}

fn main() {
    machin();
    index_first_char();
}
