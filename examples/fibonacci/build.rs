use std::{collections::BTreeMap, env, fs::File, io::Write, path::PathBuf};

use truc::{
    generator::{config::GeneratorConfig, generate},
    record::{
        definition::RecordDefinitionBuilder,
        type_resolver::{DynamicTypeInfo, StaticTypeResolver},
    },
};

const SHARED_DIR: &str = "shared_truc_examples";

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

fn build_type_resolver(cross_compilation: &CrossCompilation) -> StaticTypeResolver {
    match cross_compilation {
        CrossCompilation::No => {
            let mut resolver = StaticTypeResolver::new();
            resolver.add_all_types();
            resolver
        }
        CrossCompilation::Yes { shared_path } => {
            let file_path = shared_path.join("target_types.json");
            let json = std::fs::read_to_string(&file_path)
                .unwrap_or_else(|err| panic!("Could not read {:?}: {:?}", &file_path, err));
            let target_types: BTreeMap<String, DynamicTypeInfo> = serde_json::from_str(&json)
                .unwrap_or_else(|err| panic!("Could not parse {:?}: {:?}", &file_path, err));
            StaticTypeResolver::from(target_types)
        }
    }
}

fn main() {
    let BuildInfo {
        out_dir_path,
        cross_compilation,
    } = get_build_info();

    let type_resolver = build_type_resolver(&cross_compilation);

    let mut definition = RecordDefinitionBuilder::new(&type_resolver);

    // We'll compute the fibonacci number iteratively
    let fibo_iter_id = definition.add_datum_allow_uninit::<usize, _>("fibo_iter");
    definition.close_record_variant();

    // We'll also compute the fibonacci number by rounding
    let fibo_rounding_id = definition.add_datum_allow_uninit::<usize, _>("fibo_rounding");
    definition.close_record_variant();

    // Remove the values
    definition.remove_datum(fibo_iter_id);
    definition.remove_datum(fibo_rounding_id);

    // We'll write a boolean and a message
    definition.add_datum_allow_uninit::<bool, _>("ok");
    definition.add_datum::<String, _>("msg");
    definition.close_record_variant();

    let definition = definition.build();

    let mut file = File::create(out_dir_path.join("fibonacci_truc.rs")).unwrap();
    write!(
        file,
        "{}",
        generate(&definition, &GeneratorConfig::default())
    )
    .unwrap();
}
