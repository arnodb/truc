use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{generate_data_out_record, generate_data_record, RecordInfo, UninitKind};

pub struct FromPreviousRecordDataRecordsGenerator;

impl FragmentGenerator for FromPreviousRecordDataRecordsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let prev_record_spec = if let Some(prev_record_spec) = specs.prev_record.as_ref() {
            prev_record_spec
        } else {
            return;
        };
        let record_spec = &specs.record;

        for (record_info, uninit_kind) in [
            (
                RecordInfo {
                    name: &record_spec.unpacked_record_in_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`]."#,
                        prev_record_spec.variant.id()
                    )),
                },
                UninitKind::False,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_record_in_name,
                    public: true,
                    doc: Some(&format!(
                        r#"Data container for conversion from [`Record{}`] without the data to be left uninitialized."#,
                        prev_record_spec.variant.id()
                    )),
                },
                UninitKind::Unsafe,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_safe_record_in_name,
                    public: false,
                    doc: Some(
                        r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                    ),
                },
                UninitKind::Safe {
                    unsafe_record_name: &record_spec.unpacked_uninit_record_in_name,
                    safe_generic: record_spec.plus_uninit_safe_generic.as_ref(),
                },
            ),
        ] {
            generate_data_record(record_info, &record_spec.plus_data, uninit_kind, scope);
        }

        generate_data_out_record(
            RecordInfo {
                name: &record_spec.record_and_unpacked_out_name,
                public: true,
                doc: Some(&format!(
                    r#"Result of conversion from record variant #{} to variant #{} via a [`From::from`] call.

It contains all the removed data so that one can still use them, or drop them."#,
                    prev_record_spec.variant.id(),
                    record_spec.variant.id()
                )),
            },
            &record_spec.capped_record_name,
            &record_spec.minus_data,
            scope,
        );
    }
}
