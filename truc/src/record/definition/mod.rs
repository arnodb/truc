//! Record related structures.

use std::{
    fmt::{Debug, Display, Formatter},
    ops::Index,
};

use itertools::Itertools;

use crate::record::type_resolver::TypeInfo;

pub mod builder;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct DatumId(usize);

impl Debug for DatumId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(PartialEq, Eq, Debug, new)]
pub struct DatumDefinition<D> {
    id: DatumId,
    name: String,
    details: D,
}

impl<D> DatumDefinition<D> {
    pub fn id(&self) -> DatumId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn details(&self) -> &D {
        &self.details
    }

    pub fn details_mut(&mut self) -> &mut D {
        &mut self.details
    }
}

impl<D: Display> Display for DatumDefinition<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.id, self.name, self.details)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DatumDefinitionCollection<D> {
    data: Vec<DatumDefinition<D>>,
}

impl<D> Default for DatumDefinitionCollection<D> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<D> DatumDefinitionCollection<D> {
    fn iter(&self) -> impl Iterator<Item = &DatumDefinition<D>> {
        self.data.iter()
    }

    fn get(&self, id: DatumId) -> Option<&DatumDefinition<D>> {
        self.data.get(id.0)
    }

    fn get_mut(&mut self, id: DatumId) -> Option<&mut DatumDefinition<D>> {
        self.data.get_mut(id.0)
    }

    fn push(&mut self, name: String, details: D) -> DatumId {
        let id = DatumId::from(self.data.len());
        let datum = DatumDefinition::new(id, name, details);
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
pub struct RecordDefinition<D> {
    datum_definitions: DatumDefinitionCollection<D>,
    variants: Vec<RecordVariant>,
}

impl<D> RecordDefinition<D> {
    pub fn datum_definitions(&self) -> impl Iterator<Item = &DatumDefinition<D>> {
        self.datum_definitions.iter()
    }

    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition<D>> {
        self.datum_definitions.get(id)
    }

    pub fn variants(&self) -> impl Iterator<Item = &RecordVariant> {
        self.variants.iter()
    }

    pub fn get_variant(&self, id: RecordVariantId) -> Option<&RecordVariant> {
        self.variants.get(id.0)
    }
}

impl<D> Index<DatumId> for RecordDefinition<D> {
    type Output = DatumDefinition<D>;

    fn index(&self, index: DatumId) -> &Self::Output {
        self.get_datum_definition(index)
            .unwrap_or_else(|| panic!("datum #{} not found", index))
    }
}

impl<D> Index<RecordVariantId> for RecordDefinition<D> {
    type Output = RecordVariant;

    fn index(&self, index: RecordVariantId) -> &Self::Output {
        self.get_variant(index)
            .unwrap_or_else(|| panic!("variant #{} not found", index))
    }
}

#[derive(PartialEq, Eq, Debug, new)]
pub struct NativeDatumDetails {
    offset: usize,
    type_info: TypeInfo,
    allow_uninit: bool,
}

impl Display for NativeDatumDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, align {}, offset {}, size {}",
            self.type_info.name, self.type_info.align, self.offset, self.type_info.size
        )?;
        Ok(())
    }
}

impl NativeDatumDetails {
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

impl RecordDefinition<NativeDatumDetails> {
    pub fn max_type_align(&self) -> usize {
        self.datum_definitions()
            .map(|d| d.details().type_align())
            .reduce(usize::max)
            .unwrap_or(std::mem::align_of::<()>())
    }

    pub fn max_size(&self) -> usize {
        self.datum_definitions()
            .map(|d| d.details().offset() + d.details().size())
            .max()
            .unwrap_or(0)
    }

    fn fmt_variant_representation(
        variant: &RecordVariant,
        datum_definitions: &DatumDefinitionCollection<NativeDatumDetails>,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{} [", variant.id)?;
        let mut first = true;
        let mut byte_offset = 0;
        for &d in &variant.data {
            if !first {
                write!(f, ", ")?;
            }
            let datum = datum_definitions
                .get(d)
                .unwrap_or_else(|| panic!("datum #{}", d));
            if byte_offset > datum.details().offset() {
                panic!(
                    "offset clash {} > {}",
                    byte_offset,
                    datum.details().offset()
                );
            }
            if byte_offset < datum.details().offset() {
                write!(f, "(void, {}), ", datum.details().offset() - byte_offset)?;
            }
            write!(f, "{}", datum)?;
            first = false;
            byte_offset = datum.details().offset() + datum.details().size();
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Display for RecordDefinition<NativeDatumDetails> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for v in &self.variants {
            Self::fmt_variant_representation(v, &self.datum_definitions, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use crate::record::{
        definition::builder::native::NativeRecordDefinitionBuilder, type_resolver::HostTypeResolver,
    };

    #[test]
    fn should_display_definition() {
        let type_resolver = HostTypeResolver;
        let mut definition = NativeRecordDefinitionBuilder::new(&type_resolver);
        let uint_32_id = definition.add_datum::<u32, _>("uint_32").unwrap();
        definition.add_datum::<u16, _>("uint_16").unwrap();
        definition.close_record_variant();
        definition.remove_datum(uint_32_id).unwrap();
        definition.close_record_variant();
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
