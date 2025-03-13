use std::io::Write;

use clap::Parser;
use machin_data::MachinEnum;
use truc::record::type_resolver::StaticTypeResolver;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, value_parser, value_name = "OUTPUT")]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut type_infos = StaticTypeResolver::new();

    type_infos.add_all_types();

    type_infos.add_type::<MachinEnum>();

    let content = type_infos.to_json_string_pretty().unwrap();
    if let Some(output) = args.output {
        let mut output = std::fs::File::create(output).unwrap();
        writeln!(output, "{}", content).unwrap();
    } else {
        println!("{}", content);
    }
}
