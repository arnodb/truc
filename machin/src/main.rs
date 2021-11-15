#[macro_use]
extern crate assert_matches;
#[macro_use]
extern crate derive_new;

fn machin() {
    use machin_data::MachinEnum;
    use machin_truc::*;

    let record_4 = Record4::<MAX_SIZE>::new(NewRecord4 {
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

    let mut record_4 = Record4::<MAX_SIZE>::from(NewRecordUninit4 {
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

    let mut record_0 = Record0::<MAX_SIZE>::new(NewRecord0 {
        datum_a: 1,
        datum_b: 2,
    });

    assert_eq!(*record_0.datum_a(), 1);
    assert_eq!(*record_0.datum_b(), 2);

    let datum_c = *record_0.datum_a() + *record_0.datum_b();
    *record_0.datum_a_mut() = 42;

    let mut record_1 = Record1::from((record_0, RecordIn1 { datum_c }));

    assert_eq!(*record_1.datum_a(), 42);
    assert_eq!(*record_1.datum_b(), 2);
    assert_eq!(*record_1.datum_c(), 3);

    *record_1.datum_b_mut() = 12;

    let Record2AndOut {
        record: record_2,
        datum_a,
    } = Record2AndOut::from((record_1, RecordIn2 {}));

    assert_eq!(datum_a, 42);

    assert_eq!(*record_2.datum_b(), 12);
    assert_eq!(*record_2.datum_c(), 3);

    let record_3 = Record3::from((
        record_2,
        RecordIn3 {
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
        RecordIn4 {
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
        RecordIn5 {
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

fn index_first_char() -> Result<(), String> {
    use machin_truc::index_first_char::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Reader {
        buffer: String,
    }

    impl Iterator for Reader {
        type Item = Result<Record0<MAX_SIZE>, String>;

        fn next(&mut self) -> Option<Self::Item> {
            self.buffer.clear();
            let read = match std::io::stdin().read_line(&mut self.buffer) {
                Ok(read) => read,
                Err(err) => return Some(Err(err.to_string())),
            };
            if read > 0 {
                let words = self.buffer.trim().to_string();
                self.buffer.clear();
                Some(Ok(Record0::new(NewRecord0 { words })))
            } else {
                None
            }
        }
    }

    #[derive(new)]
    struct Splitter<I: Iterator<Item = Result<Record0<MAX_SIZE>, String>>> {
        input: I,
        #[new(default)]
        buffer: VecDeque<String>,
    }

    impl<I: Iterator<Item = Result<Record0<MAX_SIZE>, String>>> Iterator for Splitter<I> {
        type Item = Result<Record1<MAX_SIZE>, String>;

        fn next(&mut self) -> Option<Self::Item> {
            self.buffer.pop_front().map_or_else(
                || loop {
                    let record_0 = match self.input.next() {
                        Some(Ok(rec)) => rec,
                        None => return None,
                        Some(Err(err)) => return Some(Err(err)),
                    };
                    let mut words = record_0.words().split_whitespace();
                    if let Some(first) = words.next() {
                        for w in words {
                            self.buffer.push_back(w.to_string())
                        }
                        return Some(Ok(Record1::new(NewRecord1 {
                            word: first.to_string(),
                        })));
                    }
                },
                |word| Some(Ok(Record1::new(NewRecord1 { word }))),
            )
        }
    }

    #[derive(new)]
    struct AddFirstChar<I: Iterator<Item = Result<Record1<MAX_SIZE>, String>>> {
        input: I,
    }

    impl<I: Iterator<Item = Result<Record1<MAX_SIZE>, String>>> Iterator for AddFirstChar<I> {
        type Item = Result<Record2<MAX_SIZE>, String>;

        fn next(&mut self) -> Option<Self::Item> {
            let record_1 = match self.input.next() {
                Some(Ok(rec)) => rec,
                None => return None,
                Some(Err(err)) => return Some(Err(err)),
            };
            let first_char = record_1.word().chars().next();
            Some(Ok(Record2::from((record_1, RecordIn2 { first_char }))))
        }
    }

    #[derive(new)]
    struct Sort<I: Iterator<Item = Result<Record2<MAX_SIZE>, String>>> {
        input: I,
        #[new(default)]
        buffer: Vec<Record2<MAX_SIZE>>,
        #[new(value = "false")]
        finalizing: bool,
    }

    impl<I: Iterator<Item = Result<Record2<MAX_SIZE>, String>>> Iterator for Sort<I> {
        type Item = Result<Record2<MAX_SIZE>, String>;

        fn next(&mut self) -> Option<Self::Item> {
            if !self.finalizing {
                for rec in &mut self.input {
                    let record_2 = match rec {
                        Ok(rec) => rec,
                        err @ Err(_) => return Some(err),
                    };
                    self.buffer.push(record_2);
                }
                self.buffer
                    .sort_by(|a, b| a.first_char().cmp(b.first_char()).reverse());
                self.finalizing = true;
            }
            self.buffer.pop().map(Ok)
        }
    }

    for word in Sort::new(AddFirstChar::new(Splitter::new(Reader::default()))) {
        let word = word?;
        if let Some(first_char) = word.first_char() {
            println!("{} - {}", first_char, word.word())
        } else {
            println!("  - {}", word.word())
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    machin();
    index_first_char()?;
    Ok(())
}
