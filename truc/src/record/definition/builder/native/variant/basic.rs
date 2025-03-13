use super::{align_bytes, NativeDataUpdater};
use crate::record::definition::{DatumDefinitionCollection, DatumId, NativeDatumDetails};

/// The first
/// [RecordVariantBuilder](crate::record::definition::builder::generic::variant::RecordVariantBuilder)
/// implementation.
///
/// Honestly, I am not sure how I considered this implementation good.
pub fn basic(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
) -> Vec<DatumId> {
    // Remove first to optimize space
    data.remove_data(data_to_remove.iter().cloned());

    // Then add
    let mut data_caret = 0;
    let mut byte_caret = 0;
    for &datum_id in &data_to_add {
        let datum = datum_definitions
            .get(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        while data_caret < data.len() {
            let caret_datum_id = data[data_caret];
            let caret_datum = datum_definitions
                .get(caret_datum_id)
                .unwrap_or_else(|| panic!("datum #{}", caret_datum_id));
            if caret_datum.details().offset() == byte_caret {
                data_caret += 1;
                byte_caret += caret_datum.details().size();
            } else {
                let bc = align_bytes(byte_caret, datum.details().type_align());
                if bc + datum.details().size() <= caret_datum.details().offset() {
                    byte_caret = bc;
                    break;
                } else {
                    data_caret += 1;
                    byte_caret = caret_datum.details().offset() + caret_datum.details().size();
                }
            }
        }
        byte_caret = align_bytes(byte_caret, datum.details().type_align());
        data.insert(data_caret, datum_id);
        let datum_mut = datum_definitions
            .get_mut(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));
        datum_mut.details_mut().offset = byte_caret;
    }

    data
}
