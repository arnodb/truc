//! See [GeneratorConfig](super::config::GeneratorConfig) to customize the code generation.

use codegen::Scope;

use crate::record::definition::{DatumDefinition, NativeDatumDetails, RecordVariant};

pub(crate) mod clone;
pub(crate) mod data_records;
pub(crate) mod drop_impl;
pub(crate) mod from_previous_record_data_records;
pub(crate) mod from_previous_record_impls;
pub(crate) mod from_previous_record_unnamed_fields_impls;
pub(crate) mod from_unnamed_fields_impls;
pub(crate) mod from_unpacked_record_impls;
pub(crate) mod record;
pub(crate) mod record_impl;
pub(crate) mod record_unnamed_impl;
pub(crate) mod serde;

/// Trait to implement to implement any specific fragment of record definitions.
///
/// See [GeneratorConfig](crate::generator::config::GeneratorConfig).
pub trait FragmentGenerator {
    fn imports(&self, _scope: &mut Scope) {}

    fn generate(&self, _specs: &FragmentGeneratorSpecs, _scope: &mut Scope);
}

#[derive(Debug)]
pub struct FragmentGeneratorSpecs<'a> {
    pub record: &'a RecordSpec<'a>,
    pub prev_record: Option<&'a RecordSpec<'a>>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct RecordSpec<'a> {
    pub max_type_align: usize,
    pub variant: &'a RecordVariant,
    pub capped_record_name: String,
    pub record_name: String,
    pub unpacked_record_name: String,
    pub unpacked_uninit_record_name: String,
    pub unpacked_uninit_safe_record_name: String,
    pub unpacked_record_in_name: String,
    pub unpacked_uninit_record_in_name: String,
    pub unpacked_uninit_safe_record_in_name: String,
    pub record_and_unpacked_out_name: String,
    pub data: Vec<&'a DatumDefinition<NativeDatumDetails>>,
    pub minus_data: Vec<&'a DatumDefinition<NativeDatumDetails>>,
    pub plus_data: Vec<&'a DatumDefinition<NativeDatumDetails>>,
    pub unpacked_uninit_safe_generic: Option<RecordGeneric>,
    pub plus_uninit_safe_generic: Option<RecordGeneric>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct RecordGeneric {
    pub full: String,
    pub short: String,
    pub typed: String,
}
