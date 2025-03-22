use std::collections::BTreeMap;

use super::{DatumDefinition, DatumId, RecordDefinition, RecordVariant, RecordVariantId};

/// Converts a record definition to a different record definition and provide a variants mapping.
pub fn convert_record_definition<D, A, R, C, Context>(
    quirky_definition: &RecordDefinition<D>,
    add_datum: A,
    remove_datum: R,
    close_record_variant: C,
    context: &mut Context,
) -> Result<BTreeMap<RecordVariantId, RecordVariantId>, String>
where
    A: Fn(&mut Context, &DatumDefinition<D>) -> Result<DatumId, String>,
    R: Fn(&mut Context, DatumId) -> Result<(), String>,
    C: Fn(&mut Context) -> RecordVariantId,
{
    let mut datum_ids_mapping = BTreeMap::<DatumId, DatumId>::new();
    let mut variants_mapping = BTreeMap::<RecordVariantId, RecordVariantId>::new();
    let mut prev_variant = None::<&RecordVariant>;
    for variant in quirky_definition.variants() {
        let (to_add, to_remove) = if let Some(prev_variant) = prev_variant {
            let old = prev_variant.data().collect::<Vec<_>>();
            let new = variant.data().collect::<Vec<_>>();
            let mut to_add = new.clone();
            to_add.retain(|d| !old.contains(d));
            let mut to_remove = old.clone();
            to_remove.retain(|d| !new.contains(d));
            (to_add, to_remove)
        } else {
            (variant.data().collect::<Vec<_>>(), Vec::new())
        };
        for d in to_remove {
            remove_datum(context, datum_ids_mapping[&d])?;
        }
        for d in to_add {
            let datum = &quirky_definition[d];
            let new_datum_id = add_datum(context, datum)?;
            datum_ids_mapping.insert(d, new_datum_id);
        }
        let new_variant_id = close_record_variant(context);
        // In theory they are equal in value, but we should not rely on it.
        variants_mapping.insert(variant.id(), new_variant_id);
        prev_variant = Some(variant);
    }
    Ok(variants_mapping)
}
