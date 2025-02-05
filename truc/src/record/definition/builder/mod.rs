use std::ops::Index;

use variant::RecordVariantBuilder;

use super::{
    DatumDefinition, DatumDefinitionCollection, DatumId, RecordDefinition, RecordVariant,
    RecordVariantId,
};
use crate::record::type_resolver::TypeResolver;

pub mod variant;

pub struct DatumDefinitionOverride {
    pub type_name: Option<String>,
    pub size: Option<usize>,
    pub align: Option<usize>,
    pub allow_uninit: Option<bool>,
}

/// Main structure to start building record definitions.
///
/// It needs a type resolver (see
/// [type_resolver](crate::record::type_resolver) module and especially
/// [HostTypeResolver](crate::record::type_resolver::HostTypeResolver)).
pub struct RecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    datum_definitions: DatumDefinitionCollection,
    variants: Vec<RecordVariant>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    type_resolver: R,
}

impl<R> RecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    /// Creates a new builder with a type resolver (see
    /// [type_resolver](crate::record::type_resolver) module and especially
    /// [HostTypeResolver](crate::record::type_resolver::HostTypeResolver)).
    pub fn new(type_resolver: R) -> Self {
        Self {
            datum_definitions: DatumDefinitionCollection::default(),
            variants: Vec::new(),
            data_to_add: Vec::new(),
            data_to_remove: Vec::new(),
            type_resolver,
        }
    }

    /// Adds a new datum of type `T` to the current variant.
    ///
    /// `T` does not need to be `Copy`, but if it is then consider using
    /// [add_datum_allow_uninit](Self::add_datum_allow_uninit) instead.
    pub fn add_datum<T, N>(&mut self, name: N) -> DatumId
    where
        N: Into<String>,
    {
        let datum_id = self.datum_definitions.push(
            name.into(),
            usize::MAX,
            self.type_resolver.type_info::<T>(),
            false,
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    /// Adds a new datum of type `T: Copy` to the current variant.
    ///
    /// `T` needs to be `Copy` to allow uninitialized values, if it is not `Copy` then consider
    /// using [add_datum](Self::add_datum) instead.
    pub fn add_datum_allow_uninit<T, N>(&mut self, name: N) -> DatumId
    where
        T: Copy,
        N: Into<String>,
    {
        let datum_id = self.datum_definitions.push(
            name.into(),
            usize::MAX,
            self.type_resolver.type_info::<T>(),
            true,
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    /// Adds a new datum to the current variant when type information of `T` needs to be overridden.
    ///
    /// For example if you want to add a datum of type `Vec<MyStruct>` then call it like this:
    ///
    /// ```rust
    /// # use truc::record::definition::{DatumDefinitionOverride, RecordDefinitionBuilder};
    /// # use truc::record::type_resolver::HostTypeResolver;
    /// #
    /// # let mut builder = RecordDefinitionBuilder::new(HostTypeResolver);
    /// #
    /// builder.add_datum_override::<Vec<()>, _>(
    ///     "my_vec",
    ///     DatumDefinitionOverride {
    ///         // Real type name
    ///         type_name: Some("Vec<MyStruct>".to_owned()),
    ///         // Same size
    ///         size: None,
    ///         // Same alignment rule
    ///         align: None,
    ///         // Same allow_uninit flag
    ///         allow_uninit: None,
    ///     },
    /// );
    /// ```
    pub fn add_datum_override<T, N>(
        &mut self,
        name: N,
        datum_override: DatumDefinitionOverride,
    ) -> DatumId
    where
        N: Into<String>,
    {
        let datum_id = self.datum_definitions.push(
            name.into(),
            usize::MAX,
            {
                let mut target_info = self.type_resolver.type_info::<T>();
                if let Some(type_name) = datum_override.type_name {
                    target_info.name = type_name;
                }
                if let Some(size) = datum_override.size {
                    target_info.size = size;
                }
                if let Some(align) = datum_override.align {
                    target_info.align = align;
                }
                target_info
            },
            datum_override.allow_uninit.unwrap_or(false),
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    /// Adds a new datum of dynamic type to the current variant.
    pub fn add_dynamic_datum<T, N>(&mut self, name: N, r#type: T) -> DatumId
    where
        T: AsRef<str>,
        N: Into<String>,
    {
        let dynamic_type_info = self.type_resolver.dynamic_type_info(r#type.as_ref());
        let datum_id = self.datum_definitions.push(
            name.into(),
            usize::MAX,
            dynamic_type_info.info,
            dynamic_type_info.allow_uninit,
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    /// Adds a new datum by copying an existing definition.
    pub fn copy_datum(&mut self, datum: &DatumDefinition) -> DatumId {
        let datum_id = self.datum_definitions.push(
            datum.name().into(),
            usize::MAX,
            datum.type_info().clone(),
            datum.allow_uninit(),
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    /// Remove a datum from the current variant.
    ///
    /// It panics if the operation cannot be performed or is already performed.
    pub fn remove_datum(&mut self, datum_id: DatumId) {
        if let Some(variant) = self.variants.last() {
            let index = variant.data.iter().position(|&did| did == datum_id);
            if index.is_some() {
                if self.data_to_remove.contains(&datum_id) {
                    panic!("Datum with id = {} is already removed", datum_id);
                }
                self.data_to_remove.push(datum_id);
            } else {
                let index = self.data_to_add.iter().position(|&did| did == datum_id);
                if let Some(index) = index {
                    self.data_to_add.remove(index);
                } else {
                    panic!(
                        "Could not find datum to remove in previous variant, id = {}",
                        datum_id
                    );
                }
            }
        } else {
            let index = self.data_to_add.iter().position(|&did| did == datum_id);
            if let Some(index) = index {
                self.data_to_add.remove(index);
            } else {
                panic!(
                    "Could not find datum to remove in variant being built, id = {}",
                    datum_id
                );
            }
        }
    }

    fn has_pending_changes(&self) -> bool {
        // Need to create at least one variant
        self.variants.is_empty() || !self.data_to_remove.is_empty() || !self.data_to_add.is_empty()
    }

    /// Closes the current record variant and allows starting a new one.
    pub fn close_record_variant(&mut self) -> RecordVariantId {
        self.close_record_variant_with(variant::basic)
    }

    /// Closes the current record variant and allows starting a new one.
    pub fn close_record_variant_with<Builder>(&mut self, builder: Builder) -> RecordVariantId
    where
        Builder: RecordVariantBuilder,
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
    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition> {
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
    ) -> Option<&DatumDefinition> {
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
    pub fn get_current_datum_definition_by_name(&self, name: &str) -> Option<&DatumDefinition> {
        self.get_current_data()
            .filter_map(|d| self.datum_definitions.get(d))
            .find(|datum| datum.name() == name)
    }

    /// Wraps up everything into a [RecordDefinition].
    pub fn build(mut self) -> RecordDefinition {
        if !self.data_to_add.is_empty() || !self.data_to_remove.is_empty() {
            self.close_record_variant();
        }
        debug_assert!(self.data_to_add.is_empty());
        debug_assert!(self.data_to_remove.is_empty());
        RecordDefinition {
            datum_definitions: self.datum_definitions,
            variants: self.variants,
        }
    }
}

impl<R> Index<DatumId> for RecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    type Output = DatumDefinition;

    fn index(&self, index: DatumId) -> &Self::Output {
        self.get_datum_definition(index)
            .unwrap_or_else(|| panic!("datum #{} not found", index))
    }
}

impl<R> Index<RecordVariantId> for RecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    type Output = RecordVariant;

    fn index(&self, index: RecordVariantId) -> &Self::Output {
        self.get_variant(index)
            .unwrap_or_else(|| panic!("variant #{} not found", index))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::collections::BTreeSet;

    use rand::Rng;
    use rand_chacha::rand_core::SeedableRng;

    use super::RecordDefinitionBuilder;
    use crate::record::{
        definition::{DatumDefinition, DatumId},
        type_resolver::{HostTypeResolver, TypeInfo, TypeResolver},
    };

    fn add_one<R: TypeResolver>(
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

    #[test]
    fn should_align_offsets_according_to_rust_alignment_rules() {
        let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
        println!("Seed: {:#04x?}", rng.get_seed());

        let type_resolver = HostTypeResolver;

        const MAX_DATA: usize = 32;
        for _ in 0..256 {
            let mut definition = RecordDefinitionBuilder::new(&type_resolver);
            let num_data = rng.gen_range(0..=MAX_DATA);
            let data = (0..num_data)
                .map(|i| add_one(&mut definition, &mut rng, i))
                .collect::<Vec<DatumId>>();
            definition.close_record_variant();
            let mut removed = BTreeSet::new();
            for _ in 0..(num_data / 5) {
                let index = rng.gen_range(0..data.len());
                if !removed.contains(&index) {
                    removed.insert(index);
                    definition.remove_datum(data[index]);
                }
            }
            for i in 0..(num_data / 5) {
                add_one(&mut definition, &mut rng, num_data + i);
            }
            let def = definition.build();
            let max_size = def.max_size();
            for datum in def.datum_definitions() {
                assert!(datum.offset + datum.size() <= max_size);
                assert_eq!(
                    datum.offset() % datum.type_align(),
                    0,
                    "def {} is unaligned at field {:?}",
                    def,
                    datum
                );
            }
            for v in def.variants() {
                for w in v.data.as_slice().windows(2) {
                    let datum1 = &def[w[0]];
                    let datum2 = &def[w[1]];
                    assert!(datum1.offset + datum1.size() <= datum2.offset);
                }
            }
        }
    }

    #[test]
    fn should_access_data_definition_by_name() {
        let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
        println!("Seed: {:#04x?}", rng.get_seed());

        let type_resolver = HostTypeResolver;

        const MAX_DATA: usize = 32;
        for _ in 0..256 {
            let mut definition = RecordDefinitionBuilder::new(&type_resolver);
            let first_variant_field_name = "first_variant_field";
            let first_datum_id =
                definition.add_datum_allow_uninit::<usize, _>(first_variant_field_name);
            definition.close_record_variant();
            definition.remove_datum(first_datum_id);
            let num_data = rng.gen_range(0..=MAX_DATA);
            for i in 0..num_data {
                add_one(&mut definition, &mut rng, i);
            }
            {
                assert_eq!(definition.variants.len(), 1);
                let variant_id = definition.variants[0].id;
                for i in 0..num_data {
                    let datum = definition
                        .get_variant_datum_definition_by_name(variant_id, &format!("field_{}", i));
                    assert!(datum.is_none());
                }
            }
            {
                for i in 0..num_data {
                    let datum = definition
                        .get_current_datum_definition_by_name(&format!("field_{}", i))
                        .unwrap();
                    assert_eq!(datum.name(), format!("field_{}", i));
                }
            }
            definition.close_record_variant();
            if num_data > 0 {
                let variant_id = definition.variants[1].id;
                let datum = definition
                    .get_variant_datum_definition_by_name(variant_id, first_variant_field_name);
                assert!(datum.is_none());
                let datum =
                    definition.get_current_datum_definition_by_name(first_variant_field_name);
                assert!(datum.is_none());
                for i in 0..num_data {
                    let datum = definition
                        .get_variant_datum_definition_by_name(variant_id, &format!("field_{}", i))
                        .unwrap();
                    assert_eq!(datum.name(), format!("field_{}", i));
                    let datum = definition
                        .get_current_datum_definition_by_name(&format!("field_{}", i))
                        .unwrap();
                    assert_eq!(datum.name(), format!("field_{}", i));
                }
            }
        }
    }

    #[test]
    fn should_index_data_and_variants() {
        let type_resolver = HostTypeResolver;
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum_allow_uninit::<u32, _>("uint_32");
        definition.add_datum::<u16, _>("uint_16");
        definition.close_record_variant();
        definition.remove_datum(uint_32_id);

        for datum in &definition.datum_definitions.data {
            assert_eq!(definition[datum.id].id, datum.id);
        }

        for variant in &definition.variants {
            assert_eq!(definition[variant.id].id, variant.id);
        }

        let def = definition.build();

        for datum in def.datum_definitions() {
            assert_eq!(def[datum.id].id, datum.id);
        }

        for variant in def.variants() {
            assert_eq!(def[variant.id].id, variant.id);
        }
    }

    #[test]
    fn should_copy_data() {
        let type_resolver = HostTypeResolver;
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);
        let copy_id = definition.copy_datum(&DatumDefinition {
            id: DatumId(0),
            name: "copy".to_owned(),
            offset: 1,
            type_info: TypeInfo {
                name: "foo".to_owned(),
                size: 3,
                align: 5,
            },
            allow_uninit: true,
        });
        let not_copy_id = definition.copy_datum(&DatumDefinition {
            id: DatumId(0),
            name: "not_copy".to_owned(),
            offset: 7,
            type_info: TypeInfo {
                name: "foo".to_owned(),
                size: 11,
                align: 13,
            },
            allow_uninit: false,
        });
        definition.close_record_variant();

        assert_eq!(
            &DatumDefinition {
                // ID is recomputed
                id: DatumId(0),
                name: "copy".to_owned(),
                // Offset is recomputed
                offset: 0,
                type_info: TypeInfo {
                    name: "foo".to_owned(),
                    size: 3,
                    align: 5,
                },
                allow_uninit: true,
            },
            &definition[copy_id]
        );

        assert_eq!(
            &DatumDefinition {
                // ID is recomputed
                id: DatumId(1),
                name: "not_copy".to_owned(),
                // Offset is recomputed
                offset: 13,
                type_info: TypeInfo {
                    name: "foo".to_owned(),
                    size: 11,
                    align: 13,
                },
                allow_uninit: false,
            },
            &definition[not_copy_id]
        );
    }

    #[test]
    fn should_remove_datum_added_in_first_variant() {
        let type_resolver = HostTypeResolver;
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum_allow_uninit::<u32, _>("uint_32");
        definition.remove_datum(uint_32_id);
        definition.close_record_variant();
        let def = definition.build();
        assert!(def.variants().next().is_some());
        assert_eq!(0, def.variants().next().unwrap().data_len());
    }

    #[test]
    fn should_remove_datum_added_in_second_variant() {
        let type_resolver = HostTypeResolver;
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);
        definition.close_record_variant();
        let uint_32_id = definition.add_datum_allow_uninit::<u32, _>("uint_32");
        definition.remove_datum(uint_32_id);
        definition.close_record_variant();
        let def = definition.build();
        assert!(def.variants().next().is_some());
        assert_eq!(0, def.variants().next().unwrap().data_len());
    }
}
