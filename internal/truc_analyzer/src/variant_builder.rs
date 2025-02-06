use std::collections::BTreeSet;

use average::MeanWithError;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use truc::record::{
    definition::{builder::RecordDefinitionBuilder, DatumId},
    type_resolver::{HostTypeResolver, TypeResolver},
};

pub struct Args {
    pub builder: VariantBuilder,
    pub iterations: usize,
    pub max_data: usize,
    pub max_gen: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum VariantBuilder {
    // basic
    Basic,
    // dummy
    AppendData,
    AppendDataReverse,
}

pub fn run_variant_builder_statistics(
    Args {
        builder,
        iterations,
        max_data,
        max_gen,
    }: Args,
) {
    let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
    println!("Seed: {:#04x?}", rng.get_seed());

    let type_resolver = HostTypeResolver;

    let mut rates = Vec::<Vec<f64>>::with_capacity(max_gen);
    for _ in 0..max_gen {
        rates.push(Vec::with_capacity(iterations));
    }

    for _ in 0..iterations {
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);

        let mut data = Vec::<DatumId>::new();

        for gen_rates in rates.iter_mut() {
            if !data.is_empty() {
                // Remove some existing data
                let num_data = rng.gen_range(0..=max_data);
                let mut removed = BTreeSet::new();
                for _ in 0..(num_data) {
                    let index = rng.gen_range(0..data.len());
                    if !removed.contains(&index) {
                        removed.insert(index);
                        definition.remove_datum(data[index]);
                    }
                }
            }

            // Add a random number between 0 and MAX_DATA random data
            let num_data = rng.gen_range(0..=max_data);
            data.extend((0..num_data).map(|i| add_one_datum(&mut definition, &mut rng, i)));
            definition.close_record_variant_with(match builder {
                VariantBuilder::Basic => truc::record::definition::builder::variant::basic,
                VariantBuilder::AppendData => {
                    truc::record::definition::builder::variant::append_data
                }
                VariantBuilder::AppendDataReverse => {
                    truc::record::definition::builder::variant::append_data_reverse
                }
            });

            data.clear();
            data.extend(definition.get_current_data());

            gen_rates.push(filled_rate(&definition));
        }
    }

    for (gen, gen_rates) in rates.iter().enumerate() {
        let a: MeanWithError = gen_rates.iter().collect();
        println!("gen #{}: {}", gen, a.mean());
    }
}

fn add_one_datum<R: TypeResolver>(
    definition: &mut RecordDefinitionBuilder<R>,
    rng: &mut rand_chacha::ChaCha8Rng,
    i: usize,
) -> DatumId {
    match rng.gen_range(0..4) {
        0 => definition.add_datum_allow_uninit::<u8, _>(format!("field_{}", i)),
        1 => definition.add_datum_allow_uninit::<u16, _>(format!("field_{}", i)),
        2 => definition.add_datum_allow_uninit::<u32, _>(format!("field_{}", i)),
        3 => definition.add_datum_allow_uninit::<u64, _>(format!("field_{}", i)),
        i => unreachable!("Unhandled value {}", i),
    }
}

fn filled_rate<R: TypeResolver>(definition: &RecordDefinitionBuilder<R>) -> f64 {
    let mut variant_end = 0;
    let mut filled = 0;
    for datum_id in definition.get_current_data() {
        let datum = &definition[datum_id];

        filled += datum.size();
        let end = datum.offset() + datum.size();
        if end > variant_end {
            variant_end = end;
        }
    }
    if variant_end > 0 {
        filled as f64 / variant_end as f64
    } else {
        1.0
    }
}
