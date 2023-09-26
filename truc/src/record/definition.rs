use crate::record::type_name::truc_type_name;
use std::{
    fmt::{Display, Formatter},
    mem::align_of,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct DatumId(usize);

#[derive(Debug, new)]
pub struct DatumDefinition {
    id: DatumId,
    name: String,
    offset: usize,
    size: usize,
    type_name: String,
    type_align: usize,
    allow_uninit: bool,
}

impl DatumDefinition {
    pub fn id(&self) -> DatumId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn type_align(&self) -> usize {
        self.type_align
    }

    pub fn allow_uninit(&self) -> bool {
        self.allow_uninit
    }
}

impl Display for DatumDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} ({}, align {}, offset {}, size {})",
            self.id, self.name, self.type_name, self.type_align, self.offset, self.size
        )?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct DatumDefinitionCollection {
    data: Vec<DatumDefinition>,
}

impl DatumDefinitionCollection {
    fn iter(&self) -> impl Iterator<Item = &DatumDefinition> {
        self.data.iter()
    }

    fn get(&self, id: DatumId) -> Option<&DatumDefinition> {
        self.data.get(id.0)
    }

    fn get_mut(&mut self, id: DatumId) -> Option<&mut DatumDefinition> {
        self.data.get_mut(id.0)
    }

    fn push(
        &mut self,
        name: String,
        offset: usize,
        size: usize,
        type_name: String,
        type_align: usize,
        allow_uninit: bool,
    ) -> DatumId {
        let id = DatumId::from(self.data.len());
        let datum =
            DatumDefinition::new(id, name, offset, size, type_name, type_align, allow_uninit);
        self.data.push(datum);
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct RecordVariantId(usize);

#[derive(Debug)]
pub struct RecordVariant {
    id: RecordVariantId,
    data: Vec<DatumId>,
}

impl RecordVariant {
    pub fn id(&self) -> RecordVariantId {
        self.id
    }

    pub fn data(&self) -> impl Iterator<Item = DatumId> + '_ {
        self.data.iter().copied()
    }

    fn fmt_representation(
        &self,
        datum_definitions: &DatumDefinitionCollection,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{} [", self.id)?;
        let mut first = true;
        let mut byte_offset = 0;
        for &d in &self.data {
            if !first {
                write!(f, ", ")?;
            }
            let datum = datum_definitions
                .get(d)
                .unwrap_or_else(|| panic!("datum #{}", d));
            if byte_offset > datum.offset {
                panic!("offset clash {} > {}", byte_offset, datum.offset);
            }
            if byte_offset < datum.offset {
                write!(f, "(void, {}), ", datum.offset - byte_offset)?;
            }
            write!(f, "{}", datum)?;
            first = false;
            byte_offset = datum.offset + datum.size;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Display for RecordVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [", self.id)?;
        let mut first = true;
        for d in &self.data {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", d)?;
            first = false;
        }
        write!(f, "]")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RecordDefinition {
    datum_definitions: DatumDefinitionCollection,
    variants: Vec<RecordVariant>,
}

impl RecordDefinition {
    pub fn datum_definitions(&self) -> impl Iterator<Item = &DatumDefinition> {
        self.datum_definitions.iter()
    }

    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition> {
        self.datum_definitions.get(id)
    }

    pub fn variants(&self) -> impl Iterator<Item = &RecordVariant> {
        self.variants.iter()
    }

    pub fn get_variant(&self, id: RecordVariantId) -> Option<&RecordVariant> {
        self.variants.get(id.0)
    }
}

impl Display for RecordDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for v in &self.variants {
            v.fmt_representation(&self.datum_definitions, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct DatumDefinitionOverride {
    pub type_name: Option<String>,
    pub size: Option<usize>,
    pub allow_uninit: Option<bool>,
}

pub struct RecordDefinitionBuilder {
    datum_definitions: DatumDefinitionCollection,
    variants: Vec<RecordVariant>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
}

impl RecordDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            datum_definitions: DatumDefinitionCollection::default(),
            variants: Vec::new(),
            data_to_add: Vec::new(),
            data_to_remove: Vec::new(),
        }
    }

    pub fn add_datum<T, N>(&mut self, name: N) -> DatumId
    where
        N: Into<String>,
    {
        let datum_id = self.datum_definitions.push(
            name.into(),
            std::usize::MAX,
            std::mem::size_of::<T>(),
            truc_type_name::<T>(),
            align_of::<T>(),
            false,
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    pub fn add_datum_allow_uninit<T, N>(&mut self, name: N) -> DatumId
    where
        T: Copy,
        N: Into<String>,
    {
        let datum_id = self.datum_definitions.push(
            name.into(),
            std::usize::MAX,
            std::mem::size_of::<T>(),
            truc_type_name::<T>(),
            align_of::<T>(),
            true,
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

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
            std::usize::MAX,
            datum_override.size.unwrap_or_else(std::mem::size_of::<T>),
            datum_override.type_name.unwrap_or_else(truc_type_name::<T>),
            align_of::<T>(),
            datum_override.allow_uninit.unwrap_or(false),
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    pub fn copy_datum(&mut self, datum: &DatumDefinition) -> DatumId {
        let datum_id = self.datum_definitions.push(
            datum.name().into(),
            std::usize::MAX,
            datum.size(),
            datum.type_name().to_string(),
            datum.type_align(),
            datum.allow_uninit(),
        );
        self.data_to_add.push(datum_id);
        datum_id
    }

    pub fn remove_datum(&mut self, datum_id: DatumId) {
        if let Some(variant) = self.variants.last() {
            let index = variant.data.iter().position(|&did| did == datum_id);
            if index.is_some() {
                self.data_to_remove.push(datum_id);
            } else {
                panic!(
                    "Could not find datum to remove in previous variant, id = {}",
                    datum_id
                );
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

    pub fn close_record_variant(&mut self) -> RecordVariantId {
        if !self.variants.is_empty()
            && self.data_to_remove.is_empty()
            && self.data_to_add.is_empty()
        {
            return (self.variants.len() - 1).into();
        }

        let mut data = self
            .variants
            .last()
            .map(|variant| variant.data.clone())
            .unwrap_or_default();

        // Remove first to optimize space
        for &datum_id in &self.data_to_remove {
            let index = data.iter().position(|&did| did == datum_id);
            if let Some(index) = index {
                data.remove(index);
            }
        }
        self.data_to_remove.clear();

        // Then add
        let mut data_caret = 0;
        let mut byte_caret = 0;
        let align_bytes =
            |caret: usize, align: usize| -> usize { (caret + align - 1) / align * align };
        for &datum_id in &self.data_to_add {
            let datum = self
                .datum_definitions
                .get(datum_id)
                .unwrap_or_else(|| panic!("datum #{}", datum_id));
            while data_caret < data.len() {
                let caret_datum_id = data[data_caret];
                let caret_datum = self
                    .datum_definitions
                    .get(caret_datum_id)
                    .unwrap_or_else(|| panic!("datum #{}", caret_datum_id));
                if caret_datum.offset == byte_caret {
                    data_caret += 1;
                    byte_caret += caret_datum.size;
                } else {
                    {
                        let bc = align_bytes(byte_caret, datum.type_align);
                        if bc + datum.size < caret_datum.offset {
                            byte_caret = bc;
                            break;
                        } else {
                            data_caret += 1;
                            byte_caret = caret_datum.offset + caret_datum.size;
                        }
                    }
                }
            }
            byte_caret = align_bytes(byte_caret, datum.type_align);
            data.insert(data_caret, datum_id);
            let datum_mut = self
                .datum_definitions
                .get_mut(datum_id)
                .unwrap_or_else(|| panic!("datum #{}", datum_id));
            datum_mut.offset = byte_caret;
        }
        self.data_to_add.clear();

        // And build variant
        let variant_id = self.variants.len().into();
        let variant = RecordVariant {
            id: variant_id,
            data,
        };
        self.variants.push(variant);
        variant_id
    }

    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition> {
        self.datum_definitions.get(id)
    }

    pub fn get_variant(&self, id: RecordVariantId) -> Option<&RecordVariant> {
        self.variants.get(id.0)
    }

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

impl Default for RecordDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::record::definition::RecordDefinitionBuilder;
    use rand::Rng;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn should_align_offsets_according_to_rust_alignment_rules() {
        let mut rng = rand_chacha::ChaCha8Rng::from_entropy();
        println!("Seed: {:02x?}", rng.get_seed());
        const MAX_DATA: usize = 32;
        for _ in 0..256 {
            let mut definition = RecordDefinitionBuilder::new();
            let num_data = rng.gen_range(0..=MAX_DATA);
            let add_one = |definition: &mut RecordDefinitionBuilder,
                           rng: &mut rand_chacha::ChaCha8Rng,
                           i: usize| match rng.gen_range(0..4) {
                0 => {
                    definition.add_datum::<u8, _>(format!("field_{}", i));
                }
                1 => {
                    definition.add_datum::<u16, _>(format!("field_{}", i));
                }
                2 => {
                    definition.add_datum::<u32, _>(format!("field_{}", i));
                }
                3 => {
                    definition.add_datum::<u64, _>(format!("field_{}", i));
                }
                i => unreachable!("Unhandled value {}", i),
            };
            for i in 0..num_data {
                add_one(&mut definition, &mut rng, i);
            }
            definition.close_record_variant();
            for _ in 0..(num_data / 5) {
                let index = rng.gen_range(0..definition.datum_definitions.data.len());
                definition.remove_datum(definition.datum_definitions.data[index].id());
            }
            for i in 0..(num_data / 5) {
                add_one(&mut definition, &mut rng, num_data + i);
            }
            let def = definition.build();
            for datum in def.datum_definitions() {
                assert_eq!(
                    datum.offset() % datum.type_align(),
                    0,
                    "def {} is unaligned at field {:?}",
                    def,
                    datum
                );
            }
        }
    }
}
