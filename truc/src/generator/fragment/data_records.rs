use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{generate_data_record, RecordInfo, UninitKind};

pub struct DataRecordsGenerator;

impl FragmentGenerator for DataRecordsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        for (record_info, uninit_kind) in [
            (
                RecordInfo {
                    name: &record_spec.unpacked_record_name,
                    public: true,
                    doc: Some(
                        r#"Data container for packing/unpacking records.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                    ),
                },
                UninitKind::False,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_record_name,
                    public: true,
                    doc: Some(
                        r#"Data container for packing/unpacking records without the data to be left uninitialized.

All the fields are named for the safe interoperability between the generated code and the code
using it."#,
                    ),
                },
                UninitKind::Unsafe,
            ),
            (
                RecordInfo {
                    name: &record_spec.unpacked_uninit_safe_record_name,
                    public: false,
                    doc: Some(
                        r#"It only exists to check that the uninitialized data is actually [`Copy`] at run time."#,
                    ),
                },
                UninitKind::Safe {
                    unsafe_record_name: &record_spec.unpacked_uninit_record_name,
                    safe_generic: record_spec.unpacked_uninit_safe_generic.as_ref(),
                },
            ),
        ] {
            generate_data_record(record_info, &record_spec.data, uninit_kind, scope);
        }
    }
}
