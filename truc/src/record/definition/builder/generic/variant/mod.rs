//! Building generic record variants.

use crate::record::definition::{DatumDefinitionCollection, DatumId};

mod dummy;

pub use dummy::{append_data, append_data_reverse};

/// Trait to implement in order to be used as a variant builder.
pub trait RecordVariantBuilder<D> {
    /// Builder implementation.
    ///
    /// `data` contains the IDs of all the data present in the previous variant of the record.
    ///
    /// `data_to_add` and `data_to_remove` contain the IDs of all the data to add and remove to
    /// build the new record variant.
    ///
    /// `data_to_add` is guaranteed by the caller to have no intersection with either `data` or
    /// `data_to_remove`.
    ///
    /// `data_to_remove` is guaranteed by the caller to be a subset of `data`.
    ///
    /// There is a blanket implementation to implement this trait for simple stateless functions.
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
