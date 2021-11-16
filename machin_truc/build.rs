use machin_data::MachinEnum;
use std::env;
use std::fs::File;
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
    let mut def_1 = RecordDefinitionBuilder::new();

    let words = def_1.add_datum::<String, _>("words");
    def_1.close_record_variant();

    def_1.add_datum::<String, _>("word");
    def_1.remove_datum(words);
    def_1.close_record_variant();

    def_1.add_datum::<char, _>("first_char");
    let last_variant_1 = def_1.close_record_variant();

    let def_1 = def_1.build();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);
    let mut file = File::create(&dest.join("index_first_char_1.rs")).unwrap();
    generate(&def_1, &mut file).unwrap();

    let mut def_2_group = RecordDefinitionBuilder::new();

    for d in def_1
        .get_variant(last_variant_1)
        .expect("last variant 1")
        .data()
    {
        let datum = def_1.get_datum_definition(d).expect("datum");
        if datum.name() == "first_char" {
            continue;
        }
        def_2_group.copy_datum(datum);
    }

    let def_2_group = def_2_group.build();

    let mut file = File::create(&dest.join("index_first_char_2_group.rs")).unwrap();
    generate(&def_2_group, &mut file).unwrap();

    let mut def_2 = RecordDefinitionBuilder::new();

    def_2.add_datum::<char, _>("first_char");
    def_2.add_datum_override::<Vec<()>, _>(
        "words",
        DatumDefinitionOverride {
            type_name: Some("Vec<group::Record0<{ group::MAX_SIZE }>>".to_string()),
            size: None,
            allow_uninit: None,
        },
    );

    let def_2 = def_2.build();

    let mut file = File::create(&dest.join("index_first_char_2.rs")).unwrap();
    generate(&def_2, &mut file).unwrap();
}

fn main() {
    machin();
    index_first_char();
}
