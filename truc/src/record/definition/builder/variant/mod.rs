use crate::record::definition::{DatumDefinitionCollection, DatumId};

mod basic;

pub use basic::basic;

pub trait RecordVariantBuilder {
    fn build(
        self,
        data: Vec<DatumId>,
        data_to_add: Vec<DatumId>,
        data_to_remove: Vec<DatumId>,
        datum_definitions: &mut DatumDefinitionCollection,
    ) -> Vec<DatumId>;
}

impl<F> RecordVariantBuilder for F
where
    F: FnOnce(
        Vec<DatumId>,
        Vec<DatumId>,
        Vec<DatumId>,
        &mut DatumDefinitionCollection,
    ) -> Vec<DatumId>,
{
    fn build(
        self,
        data: Vec<DatumId>,
        data_to_add: Vec<DatumId>,
        data_to_remove: Vec<DatumId>,
        datum_definitions: &mut DatumDefinitionCollection,
    ) -> Vec<DatumId> {
        self(data, data_to_add, data_to_remove, datum_definitions)
    }
}

pub trait DataUpdater {
    fn end(&self, datum_definitions: &DatumDefinitionCollection) -> usize;

    fn remove_data<I>(&mut self, datum_ids: I)
    where
        I: IntoIterator<Item = DatumId> + Clone;

    fn push_datum(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        datum_id: DatumId,
    ) -> (usize, usize);
}

impl DataUpdater for Vec<DatumId> {
    fn end(&self, datum_definitions: &DatumDefinitionCollection) -> usize {
        self.last()
            .map(|&d| {
                let datum = datum_definitions
                    .get(d)
                    .unwrap_or_else(|| panic!("datum #{}", d));
                datum.offset() + datum.size()
            })
            .unwrap_or(0)
    }

    fn remove_data<I>(&mut self, datum_ids: I)
    where
        I: IntoIterator<Item = DatumId> + Clone,
    {
        self.retain(|&datum_id| !datum_ids.clone().into_iter().any(|did| did == datum_id));
    }

    fn push_datum(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        datum_id: DatumId,
    ) -> (usize, usize) {
        let end = self.end(datum_definitions);
        let datum = datum_definitions
            .get(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        let offset = align_bytes(end, datum.type_align());
        self.push(datum_id);
        let datum_mut = datum_definitions
            .get_mut(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        datum_mut.offset = offset;
        (end, offset)
    }
}

pub(crate) fn align_bytes(caret: usize, align: usize) -> usize {
    (caret + align - 1) / align * align
}
