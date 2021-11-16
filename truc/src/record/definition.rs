use crate::record::type_name::truc_type_name;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct DatumId(usize);

#[derive(Debug, new)]
pub struct DatumDefinition {
    id: DatumId,
    name: String,
    offset: usize,
    size: usize,
    type_name: String,
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

    pub fn allow_uninit(&self) -> bool {
        self.allow_uninit
    }
}

impl Display for DatumDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}, {})", self.id, self.type_name, self.size)?;
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

    fn push(
        &mut self,
        name: String,
        offset: usize,
        size: usize,
        type_name: String,
        allow_uninit: bool,
    ) -> DatumId {
        let id = DatumId::from(self.data.len());
        let datum = DatumDefinition::new(id, name, offset, size, type_name, allow_uninit);
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
        for d in &self.data {
            if !first {
                write!(f, ", ")?;
            }
            let datum = datum_definitions.get(*d).expect("datum");
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

    pub fn variants(&self) -> impl Iterator<Item = &RecordVariant> {
        self.variants.iter()
    }

    pub fn get_variant(&self, id: RecordVariantId) -> Option<&RecordVariant> {
        self.variants.get(id.0)
    }

    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition> {
        self.datum_definitions.get(id)
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

struct RecordVariantBuilder {
    id: RecordVariantId,
    data: Vec<DatumId>,
    data_carret: usize,
    byte_carret: usize,
}

impl RecordVariantBuilder {
    fn empty(id: RecordVariantId) -> Self {
        Self {
            id,
            data: Vec::new(),
            data_carret: 0,
            byte_carret: 0,
        }
    }

    fn derive(id: RecordVariantId, from: &RecordVariantBuilder) -> Self {
        Self {
            id,
            data: from.data.clone(),
            data_carret: 0,
            byte_carret: 0,
        }
    }

    fn add_datum<T, N>(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        name: N,
    ) -> DatumId
    where
        N: Into<String>,
    {
        self.add_datum_internal(
            datum_definitions,
            name.into(),
            DatumDefinitionPayload {
                type_name: truc_type_name::<T>(),
                size: std::mem::size_of::<T>(),
                allow_uninit: false,
            },
        )
    }

    fn add_datum_allow_uninit<T, N>(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        name: N,
    ) -> DatumId
    where
        T: Copy,
        N: Into<String>,
    {
        self.add_datum_internal(
            datum_definitions,
            name.into(),
            DatumDefinitionPayload {
                type_name: truc_type_name::<T>(),
                size: std::mem::size_of::<T>(),
                allow_uninit: true,
            },
        )
    }

    fn add_datum_override<T, N>(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        name: N,
        datum_override: DatumDefinitionOverride,
    ) -> DatumId
    where
        N: Into<String>,
    {
        self.add_datum_internal(
            datum_definitions,
            name.into(),
            DatumDefinitionPayload {
                type_name: datum_override.type_name.unwrap_or_else(truc_type_name::<T>),
                size: datum_override.size.unwrap_or_else(std::mem::size_of::<T>),
                allow_uninit: datum_override.allow_uninit.unwrap_or(false),
            },
        )
    }

    fn copy_datum(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        datum: &DatumDefinition,
    ) -> DatumId {
        self.add_datum_internal(
            datum_definitions,
            datum.name().to_string(),
            DatumDefinitionPayload {
                type_name: datum.type_name().to_string(),
                size: datum.size(),
                allow_uninit: datum.allow_uninit(),
            },
        )
    }

    fn add_datum_internal(
        &mut self,
        datum_definitions: &mut DatumDefinitionCollection,
        name: String,
        payload: DatumDefinitionPayload,
    ) -> DatumId {
        let mut data_carret = self.data_carret;
        let mut byte_carret = self.byte_carret;
        while data_carret < self.data.len() {
            let carret_datum_id = self.data[data_carret];
            let datum = datum_definitions.get(carret_datum_id).expect("datum");
            if datum.offset == byte_carret {
                data_carret += 1;
                self.data_carret = data_carret;
                byte_carret += datum.size;
                self.byte_carret = byte_carret;
            } else if byte_carret + payload.size < datum.offset {
                break;
            } else {
                data_carret += 1;
                byte_carret = datum.offset + datum.size;
            }
        }

        let datum_id = datum_definitions.push(
            name,
            byte_carret,
            payload.size,
            payload.type_name,
            payload.allow_uninit,
        );
        self.data.insert(data_carret, datum_id);
        datum_id
    }

    fn remove_datum(&mut self, id: DatumId, datum_definitions: &DatumDefinitionCollection) {
        let index = self.data.iter().position(|&did| did == id);
        if let Some(index) = index {
            self.data.remove(index);
            if index < self.data_carret {
                self.data_carret = index;
                self.byte_carret = datum_definitions.get(id).expect("datum").offset;
            }
        } else {
            panic!("Could not find datum to remove, id = {}", id);
        }
    }

    fn build(self) -> RecordVariant {
        RecordVariant {
            id: self.id,
            data: self.data,
        }
    }
}

pub struct DatumDefinitionPayload {
    pub type_name: String,
    pub size: usize,
    pub allow_uninit: bool,
}

pub struct DatumDefinitionOverride {
    pub type_name: Option<String>,
    pub size: Option<usize>,
    pub allow_uninit: Option<bool>,
}

pub struct RecordDefinitionBuilder {
    datum_definitions: DatumDefinitionCollection,
    variants: Vec<RecordVariant>,
    current_variant: RecordVariantBuilder,
    variant_dirty: bool,
}

impl RecordDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            datum_definitions: DatumDefinitionCollection::default(),
            variants: Vec::new(),
            current_variant: RecordVariantBuilder::empty(0.into()),
            variant_dirty: false,
        }
    }

    pub fn add_datum<T, N>(&mut self, name: N) -> DatumId
    where
        N: Into<String>,
    {
        let id = self
            .current_variant
            .add_datum::<T, N>(&mut self.datum_definitions, name);
        self.variant_dirty = true;
        id
    }

    pub fn add_datum_allow_uninit<T, N>(&mut self, name: N) -> DatumId
    where
        T: Copy,
        N: Into<String>,
    {
        let id = self
            .current_variant
            .add_datum_allow_uninit::<T, N>(&mut self.datum_definitions, name);
        self.variant_dirty = true;
        id
    }

    pub fn add_datum_override<T, N>(
        &mut self,
        name: N,
        datum_override: DatumDefinitionOverride,
    ) -> DatumId
    where
        N: Into<String>,
    {
        self.current_variant.add_datum_override::<T, N>(
            &mut self.datum_definitions,
            name,
            datum_override,
        )
    }

    pub fn copy_datum(&mut self, datum: &DatumDefinition) -> DatumId {
        let id = self
            .current_variant
            .copy_datum(&mut self.datum_definitions, datum);
        self.variant_dirty = true;
        id
    }

    pub fn remove_datum(&mut self, id: DatumId) {
        self.current_variant
            .remove_datum(id, &self.datum_definitions);
        self.variant_dirty = true;
    }

    pub fn close_record_variant(&mut self) -> RecordVariantId {
        let next_variant =
            RecordVariantBuilder::derive((self.variants.len() + 1).into(), &self.current_variant);
        let variant = std::mem::replace(&mut self.current_variant, next_variant).build();
        let variant_id = variant.id;
        self.variants.push(variant);
        self.variant_dirty = false;
        variant_id
    }

    pub fn build(mut self) -> RecordDefinition {
        if self.variant_dirty {
            self.close_record_variant();
        }
        debug_assert!(!self.variant_dirty);
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
