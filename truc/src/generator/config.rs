use super::fragment::{
    data_records::DataRecordsGenerator, drop_impl::DropImplGenerator,
    from_previous_record_data_records::FromPreviousRecordDataRecordsGenerator,
    from_previous_record_impls::FromPreviousRecordImplsGenerator,
    from_unpacked_record_impls::FromUnpackedRecordImplsGenerator, record::RecordGenerator,
    record_impl::RecordImplGenerator, FragmentGenerator,
};

pub struct GeneratorConfig {
    pub(crate) fragment_generators: Vec<Box<dyn FragmentGenerator>>,
}

impl GeneratorConfig {
    fn common_fragment_generators() -> [Box<dyn FragmentGenerator>; 7] {
        [
            Box::new(DataRecordsGenerator),
            Box::new(RecordGenerator),
            Box::new(RecordImplGenerator),
            Box::new(DropImplGenerator),
            Box::new(FromUnpackedRecordImplsGenerator),
            Box::new(FromPreviousRecordDataRecordsGenerator),
            Box::new(FromPreviousRecordImplsGenerator),
        ]
    }

    pub fn new(fragment_generators: impl IntoIterator<Item = Box<dyn FragmentGenerator>>) -> Self {
        Self {
            fragment_generators: fragment_generators.into_iter().collect(),
        }
    }

    pub fn default_with_custom_generators(
        custom_generators: impl IntoIterator<Item = Box<dyn FragmentGenerator>>,
    ) -> Self {
        Self::new(
            Self::common_fragment_generators()
                .into_iter()
                .chain(custom_generators.into_iter()),
        )
    }
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self::new(Self::common_fragment_generators())
    }
}
