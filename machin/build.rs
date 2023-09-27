use machin_data::MachinEnum;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use truc::generator::generate;
use truc::record::definition::{DatumDefinitionOverride, RecordDefinitionBuilder};
use truc::record::type_resolver::{HostTypeResolver, StaticTypeResolver, TypeInfo, TypeResolver};

const SHARED_DIR: &str = "shared_machin";

enum CrossCompilation {
    No,
    Yes { shared_path: PathBuf },
}

struct BuildInfo {
    out_dir_path: PathBuf,
    cross_compilation: CrossCompilation,
}

fn get_build_info() -> BuildInfo {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let out_dir_path = PathBuf::from(out_dir);

    let cross_compiling = std::env::var("TRUC_CROSS").is_ok();

    let host = std::env::var("HOST").expect("HOST");
    let target = std::env::var("TARGET").expect("TARGET");

    if cross_compiling {
        println!("Cross compiling for target {} on host {}", target, host);

        let cargo_target_dir = std::env::var("CARGO_TARGET_DIR").expect("CARGO_TARGET_DIR");
        let profile = std::env::var("PROFILE").expect("PROFILE");

        let target_dir_path = PathBuf::from(cargo_target_dir);
        assert!(std::fs::metadata(&target_dir_path).unwrap().is_dir());
        let shared_path = target_dir_path.join(SHARED_DIR).join(target).join(profile);

        BuildInfo {
            cross_compilation: CrossCompilation::Yes { shared_path },
            out_dir_path,
        }
    } else {
        if host != target {
            panic!(
                "Cross compilation not detected for target {} on host {}",
                target, host
            );
        } else {
            println!("Compiling for target {}", target);
        }

        BuildInfo {
            cross_compilation: CrossCompilation::No,
            out_dir_path,
        }
    }
}

enum MixedTypeResolver {
    Host(HostTypeResolver),
    Static(StaticTypeResolver),
}

impl TypeResolver for MixedTypeResolver {
    fn type_info<T>(&self) -> TypeInfo {
        match self {
            Self::Host(resolver) => resolver.type_info::<T>(),
            Self::Static(resolver) => resolver.type_info::<T>(),
        }
    }
}

fn build_type_resolver(cross_compilation: &CrossCompilation) -> MixedTypeResolver {
    match cross_compilation {
        CrossCompilation::No => MixedTypeResolver::Host(HostTypeResolver),
        CrossCompilation::Yes { shared_path } => {
            let file_path = shared_path.join("target_types.json");
            let json = std::fs::read_to_string(&file_path)
                .unwrap_or_else(|err| panic!("Could not read {:?}: {:?}", &file_path, err));
            let target_types: BTreeMap<String, TypeInfo> = serde_json::from_str(&json)
                .unwrap_or_else(|err| panic!("Could not parse {:?}: {:?}", &file_path, err));
            MixedTypeResolver::Static(StaticTypeResolver::from(target_types))
        }
    }
}

fn machin() {
    let BuildInfo {
        out_dir_path,
        cross_compilation,
    } = get_build_info();

    let type_resolver = build_type_resolver(&cross_compilation);

    let mut definition = RecordDefinitionBuilder::new(&type_resolver);

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

    let mut file = File::create(out_dir_path.join("machin_truc.rs")).unwrap();
    write!(file, "{}", generate(&definition)).unwrap();
}

fn index_first_char() {
    let BuildInfo {
        out_dir_path,
        cross_compilation,
    } = get_build_info();

    let type_resolver = build_type_resolver(&cross_compilation);

    let mut def_1 = RecordDefinitionBuilder::new(&type_resolver);

    let words = def_1.add_datum::<Box<str>, _>("words");
    def_1.close_record_variant();

    def_1.remove_datum(words);
    let word = def_1.add_datum::<Box<str>, _>("word");
    def_1.close_record_variant();

    def_1.add_datum::<char, _>("first_char");
    def_1.close_record_variant();

    let mut def_2 = RecordDefinitionBuilder::new(&type_resolver);
    def_2.copy_datum(def_1.get_datum_definition(word).expect("datum"));
    let group = def_2.close_record_variant();

    def_1.remove_datum(word);
    def_1.add_datum_override::<Vec<()>, _>(
        "words",
        DatumDefinitionOverride {
            type_name: Some(format!("Vec<super::def_2::Record{}>", group)),
            size: None,
            align: None,
            allow_uninit: None,
        },
    );

    let def_1 = def_1.build();
    let mut file = File::create(out_dir_path.join("index_first_char_1.rs")).unwrap();
    write!(file, "{}", generate(&def_1)).unwrap();

    let def_2 = def_2.build();
    let mut file = File::create(out_dir_path.join("index_first_char_2.rs")).unwrap();
    write!(file, "{}", generate(&def_2)).unwrap();
}

fn main() {
    machin();
    index_first_char();
}
