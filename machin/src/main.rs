use serde_json::json;

#[macro_use]
extern crate assert_matches;
#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
#[allow(clippy::module_inception)]
mod truc;

fn machin() {
    use machin_data::MachinEnum;

    use crate::truc::*;

    assert_eq!(MAX_SIZE, 72);

    let record_4 = Record4::new(UnpackedRecord4 {
        datum_b: 0b_0010_0010_0010_0010_0010_0010_0010_0010,
        datum_c: 0b_0100_0100_0100_0100_0100_0100_0100_0100,
        datum_d: 0b_0101_0101,
        datum_e: 0b_0001_0001_0001_0001,
        datum_f: 0b_1000_1000_1000_1000_1000_1000_1000_1000,
        #[cfg(target_pointer_width = "16")]
        machin_enum: MachinEnum::Number(42 * 1000),
        #[cfg(target_pointer_width = "32")]
        machin_enum: MachinEnum::Number(42 * 1000 * 1000),
        #[cfg(target_pointer_width = "64")]
        machin_enum: MachinEnum::Number(42 * 1000 * 1000 * 1000),
    });

    assert_eq!(*record_4.datum_b(), 0x22222222);
    assert_eq!(*record_4.datum_c(), 0x44444444);
    assert_eq!(*record_4.datum_d(), 0x55);
    assert_eq!(*record_4.datum_e(), 0x1111);
    assert_eq!(*record_4.datum_f(), 0x88888888);
    #[cfg(target_pointer_width = "16")]
    assert_matches!(record_4.machin_enum(), &MachinEnum::Number(42000));
    #[cfg(target_pointer_width = "32")]
    assert_matches!(record_4.machin_enum(), &MachinEnum::Number(42000000));
    #[cfg(target_pointer_width = "64")]
    assert_matches!(record_4.machin_enum(), &MachinEnum::Number(42000000000));

    let mut record_4 = Record4::from(UnpackedUninitRecord4 {
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
    let text = assert_matches!(
        record_4.machin_enum(),
        MachinEnum::Text(text) => text
    );
    assert_eq!(text.as_str(), "Hello World!");

    let mut record_0 = Record0::new(UnpackedRecord0 {
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
    let text = assert_matches!(
        record_4.machin_enum(),
        MachinEnum::Text(text) => text
    );
    assert_eq!(text.as_str(), "Foo");

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

fn serialize_deserialize() {
    use crate::truc::serialize_deserialize::*;

    let record_0 = Record0::new(UnpackedRecord0 {
        datum_a: 1,
        datum_b: 2,
    });

    let record_0_json = serde_json::to_value(&record_0).unwrap();
    assert_eq!(record_0_json, json!([1, 2]));

    let record_0: Record0 = serde_json::from_value(record_0_json).unwrap();
    assert_eq!(*record_0.datum_a(), 1);
    assert_eq!(*record_0.datum_b(), 2);

    let record_1 = Record1::from((record_0, UnpackedRecordIn1 { datum_c: 3 }));

    let record_1_json = serde_json::to_value(&record_1).unwrap();
    assert_eq!(record_1_json, json!([1, 2, 3]));

    let record_1: Record1 = serde_json::from_value(record_1_json).unwrap();
    assert_eq!(*record_1.datum_a(), 1);
    assert_eq!(*record_1.datum_b(), 2);
    assert_eq!(*record_1.datum_c(), 3);

    let record_2 = Record2::from((record_1, UnpackedRecordIn2 {}));

    let record_2_json = serde_json::to_value(&record_2).unwrap();
    assert_eq!(record_2_json, json!([2, 3]));

    let record_2: Record2 = serde_json::from_value(record_2_json).unwrap();
    assert_eq!(*record_2.datum_b(), 2);
    assert_eq!(*record_2.datum_c(), 3);

    let record_3 = Record3::from((
        record_2,
        UnpackedRecordIn3 {
            datum_v: vec![2, 12, 42],
        },
    ));

    let record_3_json = serde_json::to_value(&record_3).unwrap();
    assert_eq!(record_3_json, json!([2, 3, [2, 12, 42]]));

    let record_3: Record3 = serde_json::from_value(record_3_json).unwrap();
    assert_eq!(*record_3.datum_b(), 2);
    assert_eq!(*record_3.datum_c(), 3);
    assert_eq!(*record_3.datum_v(), vec![2, 12, 42]);
}

fn main() -> Result<(), String> {
    machin();
    serialize_deserialize();
    Ok(())
}
