use std::ops::Index;

use super::generic::{variant::RecordVariantBuilder, GenericRecordDefinitionBuilder};
use crate::record::{
    definition::{
        DatumDefinition, DatumId, NativeDatumDetails, RecordDefinition, RecordVariant,
        RecordVariantId,
    },
    type_resolver::TypeResolver,
};

pub mod variant;

pub struct NativeRecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    inner: GenericRecordDefinitionBuilder<NativeDatumDetails>,
    type_resolver: R,
}

impl<R> NativeRecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    /// Creates a new builder with a type resolver (see
    /// [type_resolver](crate::record::type_resolver) module and especially
    /// [HostTypeResolver](crate::record::type_resolver::HostTypeResolver)).
    pub fn new(type_resolver: R) -> Self {
        Self {
            inner: GenericRecordDefinitionBuilder::new(),
            type_resolver,
        }
    }

    /// Adds a new datum of type `T` to the current variant.
    ///
    /// `T` does not need to be `Copy`, but if it is then consider using
    /// [add_datum_allow_uninit](Self::add_datum_allow_uninit) instead.
    pub fn add_datum<T, N>(&mut self, name: N) -> Result<DatumId, String>
    where
        N: Into<String>,
    {
        self.inner.add_datum(
            name,
            NativeDatumDetails {
                offset: usize::MAX,
                type_info: self.type_resolver.type_info::<T>(),
                allow_uninit: false,
            },
        )
    }

    /// Adds a new datum of type `T: Copy` to the current variant.
    ///
    /// `T` needs to be `Copy` to allow uninitialized values, if it is not `Copy` then consider
    /// using [add_datum](Self::add_datum) instead.
    pub fn add_datum_allow_uninit<T, N>(&mut self, name: N) -> Result<DatumId, String>
    where
        T: Copy,
        N: Into<String>,
    {
        self.inner.add_datum(
            name,
            NativeDatumDetails {
                offset: usize::MAX,
                type_info: self.type_resolver.type_info::<T>(),
                allow_uninit: true,
            },
        )
    }

    /// Adds a new datum to the current variant when type information of `T` needs to be overridden.
    ///
    /// For example if you want to add a datum of type `Vec<MyStruct>` then call it like this:
    ///
    /// ```rust
    /// # use truc::record::definition::builder::native::{DatumDefinitionOverride, NativeRecordDefinitionBuilder};
    /// # use truc::record::type_resolver::HostTypeResolver;
    /// #
    /// # let mut builder = NativeRecordDefinitionBuilder::new(HostTypeResolver);
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
    ) -> Result<DatumId, String>
    where
        N: Into<String>,
    {
        self.inner.add_datum(
            name,
            NativeDatumDetails {
                offset: usize::MAX,
                type_info: {
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
                allow_uninit: datum_override.allow_uninit.unwrap_or(false),
            },
        )
    }

    /// Adds a new datum of dynamic type to the current variant.
    pub fn add_dynamic_datum<T, N>(&mut self, name: N, r#type: T) -> Result<DatumId, String>
    where
        T: AsRef<str>,
        N: Into<String>,
    {
        let dynamic_type_info = self.type_resolver.dynamic_type_info(r#type.as_ref());
        self.inner.add_datum(
            name,
            NativeDatumDetails {
                offset: usize::MAX,
                type_info: dynamic_type_info.info,
                allow_uninit: dynamic_type_info.allow_uninit,
            },
        )
    }

    /// Adds a new datum by copying an existing definition.
    pub fn copy_datum(
        &mut self,
        datum: &DatumDefinition<NativeDatumDetails>,
    ) -> Result<DatumId, String> {
        self.inner.add_datum(
            datum.name(),
            NativeDatumDetails {
                offset: usize::MAX,
                type_info: datum.details().type_info().clone(),
                allow_uninit: datum.details().allow_uninit(),
            },
        )
    }

    /// Remove a datum from the current variant.
    pub fn remove_datum(&mut self, datum_id: DatumId) -> Result<(), String> {
        self.inner.remove_datum(datum_id)
    }

    /// Closes the current record variant and allows starting a new one.
    pub fn close_record_variant(&mut self) -> RecordVariantId {
        self.close_record_variant_with(variant::simple)
    }

    /// Closes the current record variant and allows starting a new one.
    pub fn close_record_variant_with<Builder>(&mut self, builder: Builder) -> RecordVariantId
    where
        Builder: RecordVariantBuilder<NativeDatumDetails>,
    {
        self.inner.close_record_variant_with(builder)
    }

    /// Accesses datum definitions by variant ID and datum name.
    pub fn get_variant_datum_definition_by_name(
        &self,
        variant_id: RecordVariantId,
        name: &str,
    ) -> Option<&DatumDefinition<NativeDatumDetails>> {
        self.inner
            .get_variant_datum_definition_by_name(variant_id, name)
    }

    /// Accesses datum definitions of the current variant by datum name.
    pub fn get_current_data(&self) -> impl Iterator<Item = DatumId> + '_ {
        self.inner.get_current_data()
    }

    /// Accesses datum definitions of the current variant by datum name.
    pub fn get_current_datum_definition_by_name(
        &self,
        name: &str,
    ) -> Option<&DatumDefinition<NativeDatumDetails>> {
        self.inner.get_current_datum_definition_by_name(name)
    }

    /// Wraps up everything into a [RecordDefinition].
    pub fn build(self) -> RecordDefinition<NativeDatumDetails> {
        self.inner.build()
    }

    #[cfg(test)]
    pub(crate) fn inner(&self) -> &GenericRecordDefinitionBuilder<NativeDatumDetails> {
        &self.inner
    }
}

impl<R> Index<DatumId> for NativeRecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    type Output = DatumDefinition<NativeDatumDetails>;

    fn index(&self, index: DatumId) -> &Self::Output {
        &self.inner[index]
    }
}

impl<R> Index<RecordVariantId> for NativeRecordDefinitionBuilder<R>
where
    R: TypeResolver,
{
    type Output = RecordVariant;

    fn index(&self, index: RecordVariantId) -> &Self::Output {
        &self.inner[index]
    }
}

pub struct DatumDefinitionOverride {
    pub type_name: Option<String>,
    pub size: Option<usize>,
    pub align: Option<usize>,
    pub allow_uninit: Option<bool>,
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::collections::BTreeSet;

    use pretty_assertions::assert_eq;
    use rand::Rng;
    use rand_chacha::rand_core::SeedableRng;
    use rstest::rstest;

    use super::{
        variant::{self},
        NativeRecordDefinitionBuilder,
    };
    use crate::record::{
        definition::{
            builder::generic::variant::RecordVariantBuilder, DatumDefinition, DatumId,
            NativeDatumDetails,
        },
        type_resolver::{HostTypeResolver, TypeInfo, TypeResolver},
    };

    fn add_one<R: TypeResolver>(
        definition: &mut NativeRecordDefinitionBuilder<R>,
        rng: &mut rand_chacha::ChaCha8Rng,
        i: usize,
    ) -> Result<DatumId, String> {
        match rng.gen_range(0..4) {
            0 => definition.add_datum::<u8, _>(format!("field_{}", i)),
            1 => definition.add_datum::<u16, _>(format!("field_{}", i)),
            2 => definition.add_datum::<u32, _>(format!("field_{}", i)),
            3 => definition.add_datum::<u64, _>(format!("field_{}", i)),
            i => unreachable!("Unhandled value {}", i),
        }
    }

    #[rstest]
    #[case::simple(variant::simple)]
    #[case::basic(variant::basic)]
    #[case::append_data(variant::append_data)]
    #[case::append_data_reverse(variant::append_data_reverse)]
    fn should_align_offsets_according_to_rust_alignment_rules<Builder>(
        #[case] variant_builder: Builder,
    ) where
        Builder: RecordVariantBuilder<NativeDatumDetails> + Clone,
    {
        let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
        println!("Seed: {:#04x?}", rng.get_seed());

        let type_resolver = HostTypeResolver;

        const MAX_DATA: usize = 32;
        for _ in 0..256 {
            let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
            let num_data = rng.gen_range(0..=MAX_DATA);
            let data = (0..num_data)
                .map(|i| add_one(&mut definition, &mut rng, i).unwrap())
                .collect::<Vec<DatumId>>();
            definition.close_record_variant_with(variant_builder.clone());
            let mut removed = BTreeSet::new();
            for _ in 0..(num_data / 5) {
                let index = rng.gen_range(0..data.len());
                if !removed.contains(&index) {
                    removed.insert(index);
                    definition.remove_datum(data[index]).unwrap();
                }
            }
            for i in 0..(num_data / 5) {
                add_one(&mut definition, &mut rng, num_data + i).unwrap();
            }
            // Explicitely close the variant with custom variant builder
            definition.close_record_variant_with(variant_builder.clone());
            let def = definition.build();
            let max_size = def.max_size();
            for datum in def.datum_definitions() {
                assert!(datum.details().offset + datum.details().size() <= max_size);
                assert_eq!(
                    datum.details().offset() % datum.details().type_align(),
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
                    assert!(
                        datum1.details().offset() + datum1.details().size()
                            <= datum2.details().offset()
                    );
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
            let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
            let first_variant_field_name = "first_variant_field";
            let first_datum_id = definition
                .add_datum::<usize, _>(first_variant_field_name)
                .unwrap();
            definition.close_record_variant();
            definition.remove_datum(first_datum_id).unwrap();
            let num_data = rng.gen_range(0..=MAX_DATA);
            for i in 0..num_data {
                add_one(&mut definition, &mut rng, i).unwrap();
            }
            {
                assert_eq!(definition.inner().variants().len(), 1);
                let variant_id = definition.inner().variants()[0].id;
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
                let variant_id = definition.inner().variants()[1].id;
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
        let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum::<u32, _>("uint_32").unwrap();
        definition.add_datum::<u16, _>("uint_16").unwrap();
        definition.close_record_variant();
        definition.remove_datum(uint_32_id).unwrap();
        definition.close_record_variant();

        for datum in &definition.inner().datum_definitions().data {
            assert_eq!(definition[datum.id].id, datum.id);
        }

        for variant in definition.inner().variants() {
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
        let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
        let copy_id = definition
            .copy_datum(&DatumDefinition {
                id: DatumId(0),
                name: "copy".to_owned(),
                details: NativeDatumDetails {
                    offset: 1,
                    type_info: TypeInfo {
                        name: "foo".to_owned(),
                        size: 3,
                        align: 5,
                    },
                    allow_uninit: true,
                },
            })
            .unwrap();
        let not_copy_id = definition
            .copy_datum(&DatumDefinition {
                id: DatumId(0),
                name: "not_copy".to_owned(),
                details: NativeDatumDetails {
                    offset: 7,
                    type_info: TypeInfo {
                        name: "foo".to_owned(),
                        size: 11,
                        align: 13,
                    },
                    allow_uninit: false,
                },
            })
            .unwrap();
        definition.close_record_variant();

        assert_eq!(
            &DatumDefinition {
                // ID is recomputed
                id: DatumId(0),
                name: "copy".to_owned(),
                details: NativeDatumDetails {
                    // Offset is recomputed
                    offset: 15,
                    type_info: TypeInfo {
                        name: "foo".to_owned(),
                        size: 3,
                        align: 5,
                    },
                    allow_uninit: true,
                },
            },
            &definition[copy_id]
        );

        assert_eq!(
            &DatumDefinition {
                // ID is recomputed
                id: DatumId(1),
                name: "not_copy".to_owned(),
                details: NativeDatumDetails {
                    // Offset is recomputed
                    offset: 0,
                    type_info: TypeInfo {
                        name: "foo".to_owned(),
                        size: 11,
                        align: 13,
                    },
                    allow_uninit: false,
                },
            },
            &definition[not_copy_id]
        );
    }

    #[test]
    fn should_remove_datum_added_in_first_variant() {
        let type_resolver = HostTypeResolver;
        let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum::<u32, _>("uint_32").unwrap();
        definition.remove_datum(uint_32_id).unwrap();
        definition.close_record_variant();
        let def = definition.build();
        assert!(def.variants().next().is_some());
        assert_eq!(0, def.variants().next().unwrap().data_len());
    }

    #[test]
    fn should_remove_datum_added_in_second_variant() {
        let type_resolver = HostTypeResolver;
        let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
        definition.close_record_variant();
        let uint_32_id = definition.add_datum::<u32, _>("uint_32").unwrap();
        definition.remove_datum(uint_32_id).unwrap();
        definition.close_record_variant();
        let def = definition.build();
        assert!(def.variants().next().is_some());
        assert_eq!(0, def.variants().next().unwrap().data_len());
    }
}
