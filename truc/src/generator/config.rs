//! Configuration of the code generation.

use crate::generator::fragment::{
    clone::CloneImplGenerator,
    from_previous_record_unnamed_fields_impls::FromPreviousRecordUnnamedFieldsImplsGenerator,
    from_unnamed_fields_impls::FromUnnamedFieldsImplsGenerator,
    record_unnamed_impl::RecordUnnamedImplGenerator, serde::SerdeImplGenerator,
};

use super::fragment::{
    data_records::DataRecordsGenerator, drop_impl::DropImplGenerator,
    from_previous_record_data_records::FromPreviousRecordDataRecordsGenerator,
    from_previous_record_impls::FromPreviousRecordImplsGenerator,
    from_unpacked_record_impls::FromUnpackedRecordImplsGenerator, record::RecordGenerator,
    record_impl::RecordImplGenerator, FragmentGenerator,
};

/// Main configuration entry point.
pub struct GeneratorConfig {
    pub(crate) fragment_generators: Vec<Box<dyn FragmentGenerator>>,
}

impl GeneratorConfig {
    /// Constructs a new configuration instance with only the specified fragment generators.
    ///
    /// The common fragment generators are not included. This constructor is merely used for
    /// testing purpose.
    pub fn new(fragment_generators: impl IntoIterator<Item = Box<dyn FragmentGenerator>>) -> Self {
        Self {
            fragment_generators: fragment_generators.into_iter().collect(),
        }
    }

    /// Extends the fragment generators with the ones passed as argument.
    pub fn with_fragment_generators(
        mut self,
        fragment_generators: impl IntoIterator<Item = Box<dyn FragmentGenerator>>,
    ) -> Self {
        self.fragment_generators.extend(fragment_generators);
        self
    }

    /// Extends the fragments generators with the common ones.
    ///
    /// This is the default set of generators.
    pub fn with_common_fragments(self) -> Self {
        self.with_fragment_generators([
            Box::new(DataRecordsGenerator),
            Box::new(RecordGenerator),
            Box::new(RecordImplGenerator),
            Box::new(DropImplGenerator),
            Box::new(FromUnpackedRecordImplsGenerator),
            Box::new(FromPreviousRecordDataRecordsGenerator),
            Box::new(FromPreviousRecordImplsGenerator),
        ] as [Box<dyn FragmentGenerator>; 7])
    }

    /// Extends the fragment generators to support unnamed fields in constructors and `From`
    /// implementations.
    pub fn with_unnamed_fields_fragments(self) -> Self {
        self.with_fragment_generators([
            Box::new(RecordUnnamedImplGenerator),
            Box::new(FromUnnamedFieldsImplsGenerator),
            Box::new(FromPreviousRecordUnnamedFieldsImplsGenerator),
        ] as [Box<dyn FragmentGenerator>; 3])
    }

    /// Extends the fragment generators to support cloning records.
    pub fn with_clone_fragments(self) -> Self {
        self.with_fragment_generators(
            [Box::new(CloneImplGenerator)] as [Box<dyn FragmentGenerator>; 1]
        )
    }

    /// Extends the fragment generators to support record serialization/deserialization.
    pub fn with_serde_fragments(self) -> Self {
        self.with_fragment_generators(
            [Box::new(SerdeImplGenerator)] as [Box<dyn FragmentGenerator>; 1]
        )
    }
}

impl Default for GeneratorConfig {
    /// Constructs a new configuration instance with only the common fragments generators.
    fn default() -> Self {
        Self {
            fragment_generators: Vec::new(),
        }
        .with_common_fragments()
    }
}
