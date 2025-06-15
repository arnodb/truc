use truc::*;

#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
#[allow(clippy::module_inception)]
mod truc {
    include!(concat!(env!("OUT_DIR"), "/truc_asm_1_truc.rs"));
}

#[no_mangle]
fn should_convert_with_noop(record_0: Record0) -> Record1 {
    let value_0: usize = *record_0.value_0();
    Record1::from((record_0, UnpackedRecordIn1 { value_1: value_0 }))
}

fn main() {
    use crate::truc::*;

    for record_1 in (0..42)
        .map(|value_0| Record0::new(UnpackedRecord0 { value_0 }))
        .map(should_convert_with_noop)
    {
        println!("{}", record_1.value_1());
    }
}
