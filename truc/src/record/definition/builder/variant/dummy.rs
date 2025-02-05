use super::DataUpdater;
use crate::record::definition::{
    builder::variant::RecordVariantBuilder, DatumDefinitionCollection, DatumId,
};

pub fn append_data(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection,
) -> Vec<DatumId> {
    data.remove_data(data_to_remove.iter().cloned());

    for &datum_id in &data_to_add {
        data.push_datum(datum_definitions, datum_id);
    }

    data
}

// Inspired by `static_assertions`.
const _: fn() = || {
    fn assert_impl_all<T: RecordVariantBuilder>(_: T) {}
    assert_impl_all(append_data);
};

pub fn append_data_reverse(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection,
) -> Vec<DatumId> {
    data.remove_data(data_to_remove.iter().cloned());

    for &datum_id in data_to_add.iter().rev() {
        data.push_datum(datum_definitions, datum_id);
    }

    data
}

// Inspired by `static_assertions`.
const _: fn() = || {
    fn assert_impl_all<T: RecordVariantBuilder>(_: T) {}
    assert_impl_all(append_data_reverse);
};
