use machin_data::MachinEnum;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use truc::generator::generate;
use truc::record::definition::{DatumDefinitionOverride, RecordDefinitionBuilder};

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
    write!(file, "{}", generate(&definition)).unwrap();
}

fn index_first_char() {
    let mut def_1 = RecordDefinitionBuilder::new();

    let words = def_1.add_datum::<Box<str>, _>("words");
    def_1.close_record_variant();

    def_1.remove_datum(words);
    let word = def_1.add_datum::<Box<str>, _>("word");
    def_1.close_record_variant();

    def_1.add_datum::<char, _>("first_char");
    def_1.close_record_variant();

    let mut def_2 = RecordDefinitionBuilder::new();
    def_2.copy_datum(def_1.get_datum_definition(word).expect("datum"));
    let group = def_2.close_record_variant();

    def_1.remove_datum(word);
    def_1.add_datum_override::<Vec<()>, _>(
        "words",
        DatumDefinitionOverride {
            type_name: Some(format!("Vec<super::def_2::Record{}>", group)),
            size: None,
            allow_uninit: None,
        },
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);

    let def_1 = def_1.build();
    let mut file = File::create(&dest.join("index_first_char_1.rs")).unwrap();
    write!(file, "{}", generate(&def_1)).unwrap();

    let def_2 = def_2.build();
    let mut file = File::create(&dest.join("index_first_char_2.rs")).unwrap();
    write!(file, "{}", generate(&def_2)).unwrap();
}

fn main() {
    machin();
    index_first_char();
}
