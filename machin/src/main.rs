#[macro_use]
extern crate assert_matches;
#[macro_use]
extern crate derive_new;

use itertools::Itertools;
use streamink::{group::SyncGroup, io::buf::LineStream, sort::SyncSort, stream::sync::SyncStream};

fn machin() {
    use machin_data::MachinEnum;
    use machin_truc::*;

    let record_4 = Record4::<MAX_SIZE>::new(UnpackedRecord4 {
        datum_b: 0b_0010_0010_0010_0010_0010_0010_0010_0010,
        datum_c: 0b_0100_0100_0100_0100_0100_0100_0100_0100,
        datum_d: 0b_0101_0101,
        datum_e: 0b_0001_0001_0001_0001,
        datum_f: 0b_1000_1000_1000_1000_1000_1000_1000_1000,
        machin_enum: MachinEnum::Number(42 * 1000 * 1000 * 1000),
    });

    assert_eq!(*record_4.datum_b(), 0x22222222);
    assert_eq!(*record_4.datum_c(), 0x44444444);
    assert_eq!(*record_4.datum_d(), 0x55);
    assert_eq!(*record_4.datum_e(), 0x1111);
    assert_eq!(*record_4.datum_f(), 0x88888888);
    assert_matches!(record_4.machin_enum(), &MachinEnum::Number(42000000000));

    let mut record_4 = Record4::<MAX_SIZE>::from(UnpackedUninitRecord4 {
        machin_enum: MachinEnum::Text("Hello World!".to_string()),
    });

    *record_4.datum_b_mut() = 0b_0010_0010_0010_0010_0010_0010_0010_0010;
    *record_4.datum_c_mut() = 0b_0100_0100_0100_0100_0100_0100_0100_0100;
    *record_4.datum_d_mut() = 0b_0101_0101;
    *record_4.datum_e_mut() = 0b_0001_0001_0001_0001;
    *record_4.datum_f_mut() = 0b_1000_1000_1000_1000_1000_1000_1000_1000;

    assert_eq!(*record_4.datum_b(), 0x22222222);
    assert_eq!(*record_4.datum_c(), 0x44444444);
    assert_eq!(*record_4.datum_d(), 0x55);
    assert_eq!(*record_4.datum_e(), 0x1111);
    assert_eq!(*record_4.datum_f(), 0x88888888);
    assert_matches!(
        record_4.machin_enum(),
        MachinEnum::Text(text) if text.as_str() == "Hello World!"
    );

    let mut record_0 = Record0::<MAX_SIZE>::new(UnpackedRecord0 {
        datum_a: 1,
        datum_b: 2,
    });

    assert_eq!(*record_0.datum_a(), 1);
    assert_eq!(*record_0.datum_b(), 2);

    let datum_c = *record_0.datum_a() + *record_0.datum_b();
    *record_0.datum_a_mut() = 42;

    let mut record_1 = Record1::from((record_0, UnpackedRecordIn1 { datum_c }));

    assert_eq!(*record_1.datum_a(), 42);
    assert_eq!(*record_1.datum_b(), 2);
    assert_eq!(*record_1.datum_c(), 3);

    *record_1.datum_b_mut() = 12;

    let Record2AndUnpackedOut {
        record: record_2,
        datum_a,
    } = Record2AndUnpackedOut::from((record_1, UnpackedRecordIn2 {}));

    assert_eq!(datum_a, 42);

    assert_eq!(*record_2.datum_b(), 12);
    assert_eq!(*record_2.datum_c(), 3);

    let record_3 = Record3::from((
        record_2,
        UnpackedRecordIn3 {
            datum_d: 4,
            datum_e: 5,
            datum_f: 6,
        },
    ));

    assert_eq!(*record_3.datum_b(), 12);
    assert_eq!(*record_3.datum_c(), 3);
    assert_eq!(*record_3.datum_d(), 4);
    assert_eq!(*record_3.datum_e(), 5);
    assert_eq!(*record_3.datum_f(), 6);

    let record_4 = Record4::from((
        record_3,
        UnpackedRecordIn4 {
            machin_enum: MachinEnum::Text("Foo".to_string()),
        },
    ));

    assert_eq!(*record_4.datum_b(), 12);
    assert_eq!(*record_4.datum_c(), 3);
    assert_eq!(*record_4.datum_d(), 4);
    assert_eq!(*record_4.datum_e(), 5);
    assert_eq!(*record_4.datum_f(), 6);
    assert_matches!(
        record_4.machin_enum(),
        MachinEnum::Text(text) if text.as_str() == "Foo"
    );

    let record_5 = Record5::from((
        record_4,
        UnpackedRecordIn5 {
            datum_string: "Hello".to_string(),
            datum_array_of_strings: ["Hello".to_string(), "World".to_string()],
        },
    ));

    assert_eq!(record_5.datum_string(), "Hello");
    assert_eq!(
        record_5.datum_array_of_strings(),
        &["Hello".to_string(), "World".to_string()]
    );
}

pub mod ifc {
    pub mod chain_1 {
        use machin_truc::index_first_char::def_1::*;
        use std::collections::VecDeque;
        use streamink::stream::sync::SyncStream;

        #[derive(new)]
        pub struct Splitter<I: SyncStream<Item = Record0<MAX_SIZE>, Error = E>, E> {
            input: I,
            #[new(default)]
            buffer: VecDeque<Box<str>>,
            _e: std::marker::PhantomData<E>,
        }

        impl<I: SyncStream<Item = Record0<MAX_SIZE>, Error = E>, E> SyncStream for Splitter<I, E> {
            type Item = Record1<MAX_SIZE>;
            type Error = E;

            fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
                self.buffer.pop_front().map_or_else(
                    || loop {
                        let record_0 = if let Some(rec) = self.input.next()? {
                            rec
                        } else {
                            return Ok(None);
                        };
                        let mut words = record_0.words().split_whitespace();
                        if let Some(first) = words.next() {
                            for w in words {
                                self.buffer.push_back(w.into())
                            }
                            return Ok(Some(Record1::new(UnpackedRecord1 { word: first.into() })));
                        }
                    },
                    |word| Ok(Some(Record1::new(UnpackedRecord1 { word }))),
                )
            }
        }
    }
}

fn index_first_char() -> Result<(), String> {
    let input = std::io::stdin();
    let lines = LineStream::new(input.lock())
        .map_err(|err| err.to_string())
        .and_then_map(|line| -> Result<_, String> {
            Ok(machin_truc::index_first_char::def_1::Record0::<
                { machin_truc::index_first_char::def_1::MAX_SIZE },
            >::new(
                machin_truc::index_first_char::def_1::UnpackedRecord0 { words: line },
            ))
        });
    let splitted = ifc::chain_1::Splitter::new(lines).and_then_map(|rec| {
        let first_char = rec.word().chars().next().expect("first char");
        Ok(machin_truc::index_first_char::def_1::Record2::from((
            rec,
            machin_truc::index_first_char::def_1::UnpackedRecordIn2 { first_char },
        )))
    });
    let sorted = SyncSort::new(splitted, |r1, r2| {
        r1.first_char()
            .cmp(r2.first_char())
            .then_with(|| r1.word().cmp(r2.word()))
    });
    let grouped = SyncGroup::new(
        sorted,
        |rec| {
            (*rec.first_char(), {
                let machin_truc::index_first_char::def_1::UnpackedRecord2 {
                    first_char: _,
                    word,
                } = rec.unpack();
                machin_truc::index_first_char::def_2::group::Record0::<
                    { machin_truc::index_first_char::def_2::group::MAX_SIZE },
                >::new(
                    machin_truc::index_first_char::def_2::group::UnpackedRecord0 { word }
                )
            })
        },
        |first_char, group| {
            machin_truc::index_first_char::def_2::Record0::<
                { machin_truc::index_first_char::def_2::MAX_SIZE },
            >::new(machin_truc::index_first_char::def_2::UnpackedRecord0 {
                first_char,
                words: group,
            })
        },
    );
    for word in grouped.transpose() {
        let word = word?;
        println!(
            "{} - {}",
            word.first_char(),
            format!(
                "[{}]",
                word.words()
                    .iter()
                    .map(machin_truc::index_first_char::def_2::group::Record0::word)
                    .join(", ")
            )
        );
    }

    Ok(())
}

fn main() -> Result<(), String> {
    machin();
    index_first_char()?;
    Ok(())
}
