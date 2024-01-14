use codegen::Scope;

use crate::record::definition::{DatumDefinition, RecordVariant};

pub(crate) mod drop_impl;
pub(crate) mod from_previous_record_data_records;
pub(crate) mod from_previous_record_impls;
pub(crate) mod from_unpacked_record_impls;
pub(crate) mod record_impl;

pub trait FragmentGenerator {
    fn imports(&self, _scope: &mut Scope) {}

    fn generate(&self, _specs: &FragmentGeneratorSpecs, _scope: &mut Scope);
}

#[derive(Debug)]
pub struct FragmentGeneratorSpecs<'a> {
    pub record: &'a RecordSpec<'a>,
    pub prev_record: Option<RecordSpec<'a>>,
}

#[derive(Debug)]
pub struct RecordSpec<'a> {
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
    pub data: Vec<&'a DatumDefinition>,
    pub minus_data: Vec<&'a DatumDefinition>,
    pub plus_data: Vec<&'a DatumDefinition>,
    pub unpacked_uninit_safe_generic: Option<RecordGeneric>,
    pub plus_uninit_safe_generic: Option<RecordGeneric>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct RecordGeneric {
    pub full: String,
    pub short: String,
    pub typed: String,
}
