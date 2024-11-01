#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
#[allow(clippy::module_inception)]
mod truc {
    include!(concat!(env!("OUT_DIR"), "/readme_truc.rs"));
}

fn main() {
    use crate::truc::*;

    for record_2 in (0..42)
        .into_iter()
        .map(|integer| Record0::new(UnpackedRecord0 { integer }))
        .map(|mut record_0| {
            (*record_0.integer_mut()) *= 2;
            record_0
        })
        .map(|record_0| {
            let string = record_0.integer().to_string();
            Record1::from((record_0, UnpackedRecordIn1 { string }))
        })
        .map(|record_1| {
            let UnpackedRecord1 { string } = record_1.unpack();
            Record2::new(UnpackedRecord2 {
                signed_integer: string.parse().unwrap(),
            })
        })
    {
        println!("{}", record_2.signed_integer());
    }
}
