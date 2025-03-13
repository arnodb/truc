use crate::record::definition::{DatumDefinitionCollection, DatumId};

mod dummy;

pub use dummy::{append_data, append_data_reverse};

pub trait RecordVariantBuilder<D> {
    fn build(
        self,
        data: Vec<DatumId>,
        data_to_add: Vec<DatumId>,
        data_to_remove: Vec<DatumId>,
        datum_definitions: &mut DatumDefinitionCollection<D>,
    ) -> Vec<DatumId>;
}

impl<D, F> RecordVariantBuilder<D> for F
where
    F: FnOnce(
        Vec<DatumId>,
        Vec<DatumId>,
        Vec<DatumId>,
        &mut DatumDefinitionCollection<D>,
    ) -> Vec<DatumId>,
{
    fn build(
        self,
        data: Vec<DatumId>,
        data_to_add: Vec<DatumId>,
        data_to_remove: Vec<DatumId>,
        datum_definitions: &mut DatumDefinitionCollection<D>,
    ) -> Vec<DatumId> {
        self(data, data_to_add, data_to_remove, datum_definitions)
    }
}
