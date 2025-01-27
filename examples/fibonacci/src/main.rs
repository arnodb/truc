#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
#[allow(clippy::module_inception)]
mod truc;

fn main() {
    use crate::truc::*;

    let not_oks = (0..42)
        .map({
            let mut prevs = Vec::new();
            move |i| {
                let fibo_iter = match prevs.len() {
                    0 => {
                        prevs.push(0);
                        0
                    }
                    1 => {
                        prevs.push(1);
                        1
                    }
                    _ => {
                        let next = prevs.iter().sum();
                        prevs.remove(0);
                        prevs.push(next);
                        next
                    }
                };
                (
                    i,
                    // Create a new record from scratch
                    Record0::new(UnpackedRecord0 { fibo_iter }),
                )
            }
        })
        .map({
            let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
            let sqrt_five: f64 = 5.0_f64.sqrt();
            move |(i, record_0)| {
                let fibo_rounding = (phi.powi(i) / sqrt_five).round() as _;
                (
                    i,
                    // Create a new record from the old one and additional data
                    Record1::from((record_0, UnpackedRecordIn1 { fibo_rounding })),
                )
            }
        })
        .map({
            |(i, record_1)| {
                let fibo_iter = *record_1.fibo_iter();
                let fibo_rounding = *record_1.fibo_rounding();
                let ok = fibo_iter == fibo_rounding;
                // Create a new record with new data and get back the removed data for later use
                let Record2AndUnpackedOut {
                    record: record_2,
                    // Actually we don't want to reuse these, we should have unpacked the record
                    fibo_iter: _,
                    // Actually we don't want to reuse it
                    fibo_rounding: _,
                } = Record2AndUnpackedOut::from((
                    record_1,
                    UnpackedRecordIn2 {
                        ok,
                        msg: if ok {
                            format!("{}: OK {}", i, fibo_iter)
                        } else {
                            format!("{}: Not OK {} != {}", i, fibo_iter, fibo_rounding)
                        },
                    },
                ));
                (i, record_2)
            }
        })
        .filter_map(|(_i, record_2)| {
            // Unpack the record entirely
            let UnpackedRecord2 { ok, msg } = record_2.unpack();
            (!ok).then(|| msg)
        })
        .collect::<Vec<_>>();

    assert_eq!(not_oks, Vec::<String>::new());

    println!("All good!");
}
