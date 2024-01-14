use codegen::{Scope, Type};

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{CAP, CAP_GENERIC};

pub struct RecordGenerator;

impl FragmentGenerator for RecordGenerator {
    fn imports(&self, scope: &mut Scope) {
        scope.import("truc_runtime::data", "RecordMaybeUninit");
    }

    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let record = scope
            .new_struct(&record_spec.capped_record_name)
            .repr(&format!("align({})", record_spec.max_type_align))
            .vis("pub")
            .generic(CAP_GENERIC);

        let mut uninit_type = Type::new("RecordMaybeUninit");
        uninit_type.generic(CAP);
        record.field("data", &uninit_type);

        if let Some(prev_record_spec) = specs.prev_record {
            record.doc(&format!(
                r#"Record variant #{}.

It may be converted from a [`Record{}`] via one of the various call to [`From::from`]

It may also be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                        record_spec.variant.id(),
                        prev_record_spec.variant.id()
                    ));
        } else {
            record.doc(&format!(
                r#"Record variant #{}.

It may be created from initial data via one of [`new`](Self::new) or [`new_uninit`](Self::new_uninit)"#,
                        record_spec.variant.id()
                    ));
        }

        scope.raw(&format!(
            r#"/// Record variant #{} with optimized capacity.
pub type {} = {}<{{ MAX_SIZE }}>;"#,
            record_spec.variant.id(),
            record_spec.record_name,
            record_spec.capped_record_name,
        ));
    }
}
