use machin_data::MachinEnum;
use std::env;
use std::fs::File;
use std::path::Path;
use truc::generator::generate;
use truc::record::definition::RecordDefinitionBuilder;

fn main() {
    let mut definition = RecordDefinitionBuilder::new();

    let a = definition.add_datum_allow_uninit::<u32, _>("datum_a");
    let _b = definition.add_datum_allow_uninit::<u32, _>("datum_b");
    definition.close_record_variant();

    let _c = definition.add_datum_allow_uninit::<u32, _>("datum_c");
    definition.close_record_variant();

    definition.remove_datum(a);
    definition.close_record_variant();

    let _d = definition.add_datum_allow_uninit::<u8, _>("datum_d");
    let _e = definition.add_datum_allow_uninit::<u16, _>("datum_e");
    let _f = definition.add_datum_allow_uninit::<u32, _>("datum_f");
    definition.close_record_variant();

    let _g = definition.add_datum::<MachinEnum, _>("machin_enum");

    let definition = definition.build();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);
    let mut file = File::create(&dest.join("machin_truc.rs")).unwrap();
    generate(&definition, &mut file).unwrap();
}
