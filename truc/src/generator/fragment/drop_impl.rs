use codegen::Scope;

use super::{FragmentGenerator, FragmentGeneratorSpecs};
use crate::generator::{CAP, CAP_GENERIC};

pub struct DropImplGenerator;

impl FragmentGenerator for DropImplGenerator {
    fn generate(&self, specs: &FragmentGeneratorSpecs, scope: &mut Scope) {
        let record_spec = &specs.record;

        let drop_impl = scope
            .new_impl(&record_spec.capped_record_name)
            .generic(CAP_GENERIC)
            .target_generic(CAP)
            .impl_trait("Drop");

        let drop_fn = drop_impl.new_fn("drop").arg_mut_self();

        for datum in &record_spec.data {
            drop_fn.line(format!(
                "let _{}: {} = unsafe {{ self.data.read({}) }};",
                datum.name(),
                datum.type_name(),
                datum.offset(),
            ));
        }
    }
}
