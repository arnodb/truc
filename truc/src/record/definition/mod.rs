//! Record related structures.

use std::{
    fmt::{Debug, Display, Formatter},
    ops::Index,
};

use itertools::Itertools;

use crate::record::type_resolver::TypeInfo;

pub mod builder;
pub mod convert;

/// Identifier of datums (elementary data in records).
///
/// It allows identifying a datum appearing in multiple consecutive variants of a record
/// definition. Once a datum is removed from a variant, its identifier will never be readded to a
/// later variant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct DatumId(usize);

impl Debug for DatumId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

/// Generic datum definition.
///
/// Use [NativeDatumDetails] as `D` for native datum definitions.
#[derive(PartialEq, Eq, Debug, new)]
pub struct DatumDefinition<D> {
    id: DatumId,
    name: String,
    details: D,
}

impl<D> DatumDefinition<D> {
    /// Gets the identifier of this datum.
    pub fn id(&self) -> DatumId {
        self.id
    }

    /// Gets the datum name.
    ///
    /// A datum name may appear only once in a record variant. It can be used to identify a datum
    /// in a specific variant. Lookup functions are exposed in the various builders.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the details of that datum.
    ///
    /// In the case of native definitions, various Rust type related data is stored there. See
    /// [NativeDatumDetails].
    pub fn details(&self) -> &D {
        &self.details
    }

    /// Gets mutable access to the details of that datum.
    ///
    /// See [details](Self::details).
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

/// Container for datum definitions.
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

    /// Gets a datum definition by ID.
    pub fn get(&self, id: DatumId) -> Option<&DatumDefinition<D>> {
        self.data.get(id.0)
    }

    /// Gets a mutable access to a datum definition by ID.
    pub fn get_mut(&mut self, id: DatumId) -> Option<&mut DatumDefinition<D>> {
        self.data.get_mut(id.0)
    }

    fn push(&mut self, name: String, details: D) -> DatumId {
        let id = DatumId::from(self.data.len());
        let datum = DatumDefinition::new(id, name, details);
        self.data.push(datum);
        id
    }
}

/// Identifier of record variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From)]
pub struct RecordVariantId(usize);

/// Record variant definition.
#[derive(PartialEq, Eq, Debug, new)]
pub struct RecordVariant {
    id: RecordVariantId,
    data: Vec<DatumId>,
}

impl RecordVariant {
    /// Gets the identifier of this variant.
    pub fn id(&self) -> RecordVariantId {
        self.id
    }

    /// Gets access to record data in an internal order.
    ///
    /// In the case of native definitions, the order matches the order in memory and is different
    /// from the order of insertion of data.
    ///
    /// See [data_sorted](Self::data_sorted).
    pub fn data(&self) -> impl Iterator<Item = DatumId> + '_ {
        self.data.iter().copied()
    }

    /// Gets access to record data in the order of insertion.
    ///
    /// See [data](Self::data).
    pub fn data_sorted(&self) -> impl Iterator<Item = DatumId> + '_ {
        // Since datum IDs are generated with a sequence, it's enough to sort by ID to achieve
        // "order of insertion".
        self.data.iter().copied().sorted()
    }

    /// Gets the number of datums in the variant.
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

/// Record definition structure.
///
/// It is the output of record definition builders (see [builder]).
#[derive(Debug)]
pub struct RecordDefinition<D> {
    datum_definitions: DatumDefinitionCollection<D>,
    variants: Vec<RecordVariant>,
}

impl<D> RecordDefinition<D> {
    /// Gets an iterator on all datum definitions.
    pub fn datum_definitions(&self) -> impl Iterator<Item = &DatumDefinition<D>> {
        self.datum_definitions.iter()
    }

    /// Gets a datum definition by ID.
    pub fn get_datum_definition(&self, id: DatumId) -> Option<&DatumDefinition<D>> {
        self.datum_definitions.get(id)
    }

    /// Gets an iterator on all record variants.
    pub fn variants(&self) -> impl Iterator<Item = &RecordVariant> {
        self.variants.iter()
    }

    /// Gets a variant definition by ID.
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

/// Rust native datum details to use as generic in [RecordDefinition].
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
    /// Gets the offset of this datum in the record buffer.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Gets the size of this datum in the record buffer.
    pub fn size(&self) -> usize {
        self.type_info.size
    }

    /// Gets the type information of this datum.
    pub fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    /// Gets the type name of this datum.
    pub fn type_name(&self) -> &str {
        &self.type_info.name
    }

    /// Gets the type alignment of this datum.
    pub fn type_align(&self) -> usize {
        self.type_info.align
    }

    /// Gets the type `allow_uninit` flag of this datum.
    pub fn allow_uninit(&self) -> bool {
        self.allow_uninit
    }
}

impl RecordDefinition<NativeDatumDetails> {
    /// Gets the maximum value of type alignment in the definition.
    ///
    /// It is used to determine the alignment of the record structure.
    pub fn max_type_align(&self) -> usize {
        self.datum_definitions()
            .map(|d| d.details().type_align())
            .reduce(usize::max)
            .unwrap_or(std::mem::align_of::<()>())
    }

    /// Gets the maximum size of all record variants.
    ///
    /// This is used to determine the size of the byte buffer required to store any variant of this
    /// record definition.
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
