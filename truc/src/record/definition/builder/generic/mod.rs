//! Implementation of shared record definition builder behaviours.
//!
//! This implementation is generic as opposed to the one in [native](super::native) which requires
//! a type resolver in its generic definition.

use std::ops::Index;

use variant::RecordVariantBuilder;

use crate::record::definition::{
    DatumDefinition, DatumDefinitionCollection, DatumId, RecordDefinition, RecordVariant,
    RecordVariantId,
};

pub mod variant;

/// Main structure to start building generic record definitions.
///
/// [GenericRecordDefinitionBuilder] can be used to build abstract record definitions when the Rust
/// code generator is not yet necessary.
///
/// When the Rust code generator is required, one has only to replay the build process with a
/// [NativeRecordDefinitionBuilder](super::native::NativeRecordDefinitionBuilder) and perform ID
/// mapping.
pub struct GenericRecordDefinitionBuilder<D> {
    datum_definitions: DatumDefinitionCollection<D>,
    variants: Vec<RecordVariant>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
}

impl<D> GenericRecordDefinitionBuilder<D> {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new datum with details to the current variant.
    pub fn add_datum<N>(&mut self, name: N, details: D) -> Result<DatumId, String>
    where
        N: Into<String>,
    {
        let name = name.into();
        if self.get_current_datum_definition_by_name(&name).is_some() {
            return Err(format!(
                "Field with name {} already exists in current variant",
                name
            ));
        }
        let datum_id = self.datum_definitions.push(name, details);
        self.data_to_add.push(datum_id);
        Ok(datum_id)
    }

    /// Remove a datum from the current variant.
    pub fn remove_datum(&mut self, datum_id: DatumId) -> Result<(), String> {
        if let Some(variant) = self.variants.last() {
            let index = variant.data.iter().position(|&did| did == datum_id);
            if index.is_some() {
                if self.data_to_remove.contains(&datum_id) {
                    return Err(format!("Datum with id = {} is already removed", datum_id));
                }
                self.data_to_remove.push(datum_id);
            } else {
                let index = self.data_to_add.iter().position(|&did| did == datum_id);
                if let Some(index) = index {
                    self.data_to_add.remove(index);
                } else {
                    return Err(format!(
                        "Could not find datum to remove in previous variant, id = {}",
                        datum_id
                    ));
                }
            }
        } else {
            let index = self.data_to_add.iter().position(|&did| did == datum_id);
            if let Some(index) = index {
                self.data_to_add.remove(index);
            } else {
                return Err(format!(
                    "Could not find datum to remove in variant being built, id = {}",
                    datum_id
                ));
            }
        }
        Ok(())
    }

    fn has_pending_changes(&self) -> bool {
        // Need to create at least one variant
        self.variants.is_empty() || !self.data_to_remove.is_empty() || !self.data_to_add.is_empty()
    }

    /// Closes the current record variant and allows starting a new one.
    pub fn close_record_variant_with<Builder>(&mut self, builder: Builder) -> RecordVariantId
    where
        Builder: RecordVariantBuilder<D>,
    {
        if !self.has_pending_changes() {
            return (self.variants.len() - 1).into();
        }

        let data = self
            .variants
            .last()
            .map(|variant| variant.data.clone())
            .unwrap_or_default();

        let data = builder.build(
            data,
            std::mem::take(&mut self.data_to_add),
            std::mem::take(&mut self.data_to_remove),
            &mut self.datum_definitions,
        );

        // And build variant
        let variant_id = self.variants.len().into();
        let variant = RecordVariant {
            id: variant_id,
            data,
        };
        self.variants.push(variant);
        variant_id
    }

    /// Accesses datum definitions by ID.
    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition<D>> {
        self.datum_definitions.get(id)
    }

    /// Accesses variant definitions by ID.
    pub fn get_variant(&self, id: RecordVariantId) -> Option<&RecordVariant> {
        self.variants.get(id.0)
    }

    /// Accesses datum definitions by variant ID and datum name.
    pub fn get_variant_datum_definition_by_name(
        &self,
        variant_id: RecordVariantId,
        name: &str,
    ) -> Option<&DatumDefinition<D>> {
        self.get_variant(variant_id).and_then(|variant| {
            for d in variant.data() {
                let datum = self
                    .datum_definitions
                    .get(d)
                    .filter(|datum| datum.name() == name);
                if datum.is_some() {
                    return datum;
                }
            }
            None
        })
    }

    /// Gets the IDs of the data present in the current variant.
    ///
    /// It takes the removed and added data into account even when the variant is not closed yet.
    pub fn get_current_data(&self) -> impl Iterator<Item = DatumId> + '_ {
        self.variants
            .last()
            .map(|variant| {
                variant
                    .data
                    .iter()
                    .cloned()
                    .filter(|d| !self.data_to_remove.contains(d))
            })
            .into_iter()
            .flatten()
            .chain(self.data_to_add.iter().cloned())
    }

    /// Accesses datum definitions of the current variant by datum name.
    pub fn get_current_datum_definition_by_name(&self, name: &str) -> Option<&DatumDefinition<D>> {
        self.get_current_data()
            .filter_map(|d| self.datum_definitions.get(d))
            .find(|datum| datum.name() == name)
    }

    /// Wraps up everything into a [RecordDefinition].
    pub fn build(self) -> RecordDefinition<D> {
        if !self.data_to_add.is_empty() || !self.data_to_remove.is_empty() {
            panic!("The latest record variant is not closed");
        }
        RecordDefinition {
            datum_definitions: self.datum_definitions,
            variants: self.variants,
        }
    }

    #[cfg(test)]
    pub(crate) fn variants(&self) -> &[RecordVariant] {
        &self.variants
    }

    #[cfg(test)]
    pub(crate) fn datum_definitions(&self) -> &DatumDefinitionCollection<D> {
        &self.datum_definitions
    }
}

impl<D> Default for GenericRecordDefinitionBuilder<D> {
    fn default() -> Self {
        Self {
            datum_definitions: Default::default(),
            variants: Default::default(),
            data_to_add: Default::default(),
            data_to_remove: Default::default(),
        }
    }
}

impl<D> Index<DatumId> for GenericRecordDefinitionBuilder<D> {
    type Output = DatumDefinition<D>;

    fn index(&self, index: DatumId) -> &Self::Output {
        self.get_datum_definition(index)
            .unwrap_or_else(|| panic!("datum #{} not found", index))
    }
}

impl<D> Index<RecordVariantId> for GenericRecordDefinitionBuilder<D> {
    type Output = RecordVariant;

    fn index(&self, index: RecordVariantId) -> &Self::Output {
        self.get_variant(index)
            .unwrap_or_else(|| panic!("variant #{} not found", index))
    }
}
