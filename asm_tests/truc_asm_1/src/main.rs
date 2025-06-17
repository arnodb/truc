#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
#[allow(clippy::module_inception)]
mod truc {
    include!(concat!(env!("OUT_DIR"), "/truc_asm_1_truc.rs"));
}

fn main() {
    #[cfg(feature = "should_convert_with_noop")]
    {
        use crate::truc::*;

        #[no_mangle]
        fn should_convert_with_noop(record_0: Record0) -> Record1 {
            let value_0: usize = *record_0.value_0();
            Record1::from((record_0, UnpackedRecordIn1 { value_1: value_0 }))
        }

        for record_1 in (0..42)
            .map(|value_0| Record0::new(UnpackedRecord0 { value_0 }))
            .map(should_convert_with_noop)
        {
            println!("{}", record_1.value_1());
        }
    }

    #[cfg(feature = "should_convert_vec_with_noop")]
    {
        use crate::truc::*;

        #[no_mangle]
        fn should_convert_vec_with_noop(vec_0: Vec<Record0>) -> Vec<Record1> {
            use truc_runtime::convert::{convert_vec_in_place, VecElementConversionResult};
            convert_vec_in_place(vec_0, |record_0, _| {
                let value_0: usize = *record_0.value_0();
                VecElementConversionResult::Converted(Record1::from((
                    record_0,
                    UnpackedRecordIn1 { value_1: value_0 },
                )))
            })
        }

        let vec_0 = (0..42)
            .map(|value_0| Record0::new(UnpackedRecord0 { value_0 }))
            .collect::<Vec<_>>();
        should_convert_vec_with_noop(vec_0);
    }

    #[cfg(feature = "should_convert_vec_one_to_one_with_noop")]
    {
        use crate::truc::*;

        #[no_mangle]
        fn should_convert_vec_one_to_one_with_noop(vec_0: Vec<Record0>) -> Vec<Record1> {
            use truc_runtime::convert::convert_vec_in_place_one_to_one;
            convert_vec_in_place_one_to_one(vec_0, |record_0| {
                let value_0: usize = *record_0.value_0();
                Record1::from((record_0, UnpackedRecordIn1 { value_1: value_0 }))
            })
        }

        let vec_0 = (0..42)
            .map(|value_0| Record0::new(UnpackedRecord0 { value_0 }))
            .collect::<Vec<_>>();
        should_convert_vec_one_to_one_with_noop(vec_0);
    }
}
