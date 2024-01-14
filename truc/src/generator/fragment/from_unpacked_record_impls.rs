use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{RecordImplRecordNames, CAP, CAP_GENERIC};

pub struct FromUnpackedRecordImplsGenerator;

impl FromUnpackedRecordImplsGenerator {
    fn generate_from_unpacked_record_impl(
        record_names: RecordImplRecordNames,
        uninit: bool,
        scope: &mut Scope,
    ) {
        let from_impl = scope
            .new_impl(record_names.name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait(format!("From<{}>", record_names.unpacked));

        let from_fn = from_impl
            .new_fn("from")
            .arg("from", record_names.unpacked)
            .ret("Self");
        from_fn.line(format!(
            "Self::{}(from)",
            if !uninit { "new" } else { "new_uninit" },
        ));
    }
}

impl FragmentGenerator for FromUnpackedRecordImplsGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        Self::generate_from_unpacked_record_impl(
            RecordImplRecordNames {
                name: &record_spec.capped_record_name,
                unpacked: &record_spec.unpacked_record_name,
            },
            false,
            scope,
        );

        Self::generate_from_unpacked_record_impl(
            RecordImplRecordNames {
                name: &record_spec.capped_record_name,
                unpacked: &record_spec.unpacked_uninit_record_name,
            },
            true,
            scope,
        );
    }
}
