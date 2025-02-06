use clap::{Parser, ValueEnum};
use variant_builder::run_variant_builder_statistics;

mod variant_builder;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
enum CliArgs {
    VariantBuilderStatistics {
        #[clap(short, long, arg_enum)]
        builder: Option<VariantBuilder>,

        #[clap(short, long)]
        iterations: Option<usize>,

        #[clap(short = 'd', long)]
        max_data: Option<usize>,

        #[clap(short = 'g', long)]
        max_gen: Option<usize>,
    },
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum VariantBuilder {
    // basic
    Basic,
    // dummy
    AppendData,
    AppendDataReverse,
}

impl From<VariantBuilder> for variant_builder::VariantBuilder {
    fn from(value: VariantBuilder) -> Self {
        match value {
            VariantBuilder::Basic => variant_builder::VariantBuilder::Basic,
            VariantBuilder::AppendData => variant_builder::VariantBuilder::AppendData,
            VariantBuilder::AppendDataReverse => variant_builder::VariantBuilder::AppendDataReverse,
        }
    }
}

fn main() {
    let cli_args = CliArgs::parse();
    match cli_args {
        CliArgs::VariantBuilderStatistics {
            builder,
            iterations,
            max_data,
            max_gen,
        } => run_variant_builder_statistics(variant_builder::Args {
            builder: builder.unwrap_or(VariantBuilder::Basic).into(),
            iterations: iterations.unwrap_or(1024),
            max_data: max_data.unwrap_or(32),
            max_gen: max_gen.unwrap_or(16),
        }),
    }
}
