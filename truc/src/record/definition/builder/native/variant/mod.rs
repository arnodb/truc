//! Building native record variants.

use crate::record::definition::{DatumDefinitionCollection, DatumId, NativeDatumDetails};

mod basic;
mod dummy;
mod simple;

pub use basic::basic;
pub use dummy::{append_data, append_data_reverse};
pub use simple::simple;

/// Extension trait for [`Vec<DatumId>`].
pub trait NativeDataUpdater {
    /// Finds the end offset boundary of the record variant.
    ///
    /// This is where datums can be appended.
    fn end(&self, datum_definitions: &DatumDefinitionCollection<NativeDatumDetails>) -> usize;

    /// Removes a set of datums in one go.
    fn remove_data<I>(&mut self, datum_ids: I)
    where
        I: IntoIterator<Item = DatumId> + Clone;

    /// Pushes a datum at the end of the record variant.
    fn push_datum(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
        datum_id: DatumId,
    ) -> (usize, usize);
}

impl NativeDataUpdater for Vec<DatumId> {
    fn end(&self, datum_definitions: &DatumDefinitionCollection<NativeDatumDetails>) -> usize {
        self.last()
            .map(|&d| {
                let datum = datum_definitions
                    .get(d)
                    .unwrap_or_else(|| panic!("datum #{}", d));
                datum.details().offset() + datum.details().size()
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
        datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
        datum_id: DatumId,
    ) -> (usize, usize) {
        let end = self.end(datum_definitions);
        let datum = datum_definitions
            .get(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        let offset = align_bytes(end, datum.details().type_align());
        self.push(datum_id);
        let datum_mut = datum_definitions
            .get_mut(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        datum_mut.details_mut().offset = offset;
        (end, offset)
    }
}

pub(crate) fn align_bytes(caret: usize, align: usize) -> usize {
    (caret + align - 1) / align * align
}
