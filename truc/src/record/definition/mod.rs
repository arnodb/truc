//! Record related structures.

use std::{
    fmt::{Debug, Display, Formatter},
    ops::Index,
};

use itertools::Itertools;

use crate::record::type_resolver::TypeInfo;

pub mod builder;

pub use builder::{DatumDefinitionOverride, RecordDefinitionBuilder};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct DatumId(usize);

impl Debug for DatumId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(PartialEq, Eq, Debug, new)]
pub struct DatumDefinition {
    id: DatumId,
    name: String,
    offset: usize,
    type_info: TypeInfo,
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
        self.type_info.size
    }

    pub fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    pub fn type_name(&self) -> &str {
        &self.type_info.name
    }

    pub fn type_align(&self) -> usize {
        self.type_info.align
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
            self.id,
            self.name,
            self.type_info.name,
            self.type_info.align,
            self.offset,
            self.type_info.size
        )?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DatumDefinitionCollection {
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
        type_info: TypeInfo,
        allow_uninit: bool,
    ) -> DatumId {
        let id = DatumId::from(self.data.len());
        let datum = DatumDefinition::new(id, name, offset, type_info, allow_uninit);
        self.data.push(datum);
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct RecordVariantId(usize);

#[derive(PartialEq, Eq, Debug, new)]
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

    pub fn data_sorted(&self) -> impl Iterator<Item = DatumId> + '_ {
        self.data.iter().copied().sorted()
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
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
            if byte_offset > datum.offset() {
                panic!("offset clash {} > {}", byte_offset, datum.offset());
            }
            if byte_offset < datum.offset() {
                write!(f, "(void, {}), ", datum.offset() - byte_offset)?;
            }
            write!(f, "{}", datum)?;
            first = false;
            byte_offset = datum.offset() + datum.size();
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

    pub fn max_type_align(&self) -> usize {
        self.datum_definitions()
            .map(|d| d.type_align())
            .reduce(usize::max)
            .unwrap_or(std::mem::align_of::<()>())
    }

    pub fn max_size(&self) -> usize {
        self.datum_definitions()
            .map(|d| d.offset() + d.size())
            .max()
            .unwrap_or(0)
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

impl Index<DatumId> for RecordDefinition {
    type Output = DatumDefinition;

    fn index(&self, index: DatumId) -> &Self::Output {
        self.get_datum_definition(index)
            .unwrap_or_else(|| panic!("datum #{} not found", index))
    }
}

impl Index<RecordVariantId> for RecordDefinition {
    type Output = RecordVariant;

    fn index(&self, index: RecordVariantId) -> &Self::Output {
        self.get_variant(index)
            .unwrap_or_else(|| panic!("variant #{} not found", index))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::RecordDefinitionBuilder;
    use crate::record::type_resolver::HostTypeResolver;

    #[test]
    fn should_display_definition() {
        let type_resolver = HostTypeResolver;
        let mut definition = RecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum_allow_uninit::<u32, _>("uint_32");
        definition.add_datum::<u16, _>("uint_16");
        definition.close_record_variant();
        definition.remove_datum(uint_32_id);
        let def = definition.build();
        assert_eq!(
            def.to_string(),
            concat!(
                "0 [",
                "0: uint_32 (u32, align 4, offset 0, size 4), ",
                "1: uint_16 (u16, align 2, offset 4, size 2)",
                "]\n",
                "1 [",
                "(void, 4), ",
                "1: uint_16 (u16, align 2, offset 4, size 2)",
                "]\n"
            )
            .to_string()
        );
        assert_eq!(def.variants[0].to_string(), "0 [0, 1]");
        assert_eq!(def.variants[1].to_string(), "1 [1]");
    }
}
