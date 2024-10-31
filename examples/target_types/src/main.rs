use machin_data::MachinEnum;
use truc::record::type_resolver::StaticTypeResolver;

fn main() {
    let mut type_infos = StaticTypeResolver::new();

    type_infos.add_std_types();

    type_infos.add_type::<MachinEnum>();

    println!("{}", type_infos.to_json_string_pretty().unwrap());
}
