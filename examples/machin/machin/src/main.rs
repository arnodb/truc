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

    #[cfg(target_pointer_width = "64")]
    assert_eq!(MAX_SIZE, 72);
    #[cfg(target_pointer_width = "32")]
    assert_eq!(MAX_SIZE, 36);
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    unimplemented!();

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

    println!("machin OK");
}

fn serialize_deserialize_json() {
    use crate::truc::serialize_deserialize::*;

    let record_0 = Record0::new(UnpackedRecord0 {
        datum_a: 1,
        datum_b: 2,
    });

    let record_0_json = serde_json::to_value(&record_0).unwrap();
    assert_eq!(record_0_json, json!([1, 2]));

    for record_0 in [
        serde_json::from_str::<Record0>(&record_0_json.to_string()).unwrap(),
        serde_json::from_value::<Record0>(record_0_json).unwrap(),
    ] {
        assert_eq!(*record_0.datum_a(), 1);
        assert_eq!(*record_0.datum_b(), 2);
        let UnpackedRecord0 {
            datum_a: _,
            datum_b: _,
        } = record_0.unpack();
    }

    let record_1 = Record1::from((record_0, UnpackedRecordIn1 { datum_c: 3 }));

    let record_1_json = serde_json::to_value(&record_1).unwrap();
    assert_eq!(record_1_json, json!([1, 2, 3]));

    for record_1 in [
        serde_json::from_str::<Record1>(&record_1_json.to_string()).unwrap(),
        serde_json::from_value::<Record1>(record_1_json).unwrap(),
    ] {
        assert_eq!(*record_1.datum_a(), 1);
        assert_eq!(*record_1.datum_b(), 2);
        assert_eq!(*record_1.datum_c(), 3);
        let UnpackedRecord1 {
            datum_a: _,
            datum_b: _,
            datum_c: _,
        } = record_1.unpack();
    }

    let record_2 = Record2::from((record_1, UnpackedRecordIn2 {}));

    let record_2_json = serde_json::to_value(&record_2).unwrap();
    assert_eq!(record_2_json, json!([2, 3]));

    for record_2 in [
        serde_json::from_str::<Record2>(&record_2_json.to_string()).unwrap(),
        serde_json::from_value::<Record2>(record_2_json).unwrap(),
    ] {
        assert_eq!(*record_2.datum_b(), 2);
        assert_eq!(*record_2.datum_c(), 3);
        let UnpackedRecord2 {
            datum_b: _,
            datum_c: _,
        } = record_2.unpack();
    }

    let record_3 = Record3::from((
        record_2,
        UnpackedRecordIn3 {
            datum_v: vec![2, 12, 42],
        },
    ));

    let record_3_json = serde_json::to_value(&record_3).unwrap();
    assert_eq!(record_3_json, json!([2, 3, [2, 12, 42]]));

    for record_3 in [
        serde_json::from_str::<Record3>(&record_3_json.to_string()).unwrap(),
        serde_json::from_value::<Record3>(record_3_json).unwrap(),
    ] {
        assert_eq!(*record_3.datum_b(), 2);
        assert_eq!(*record_3.datum_c(), 3);
        assert_eq!(*record_3.datum_v(), vec![2, 12, 42]);
        let UnpackedRecord3 {
            datum_b: _,
            datum_c: _,
            datum_v: _,
        } = record_3.unpack();
    }

    let record_4 = Record4::from((record_3, UnpackedRecordIn4 {}));

    let record_4_json = serde_json::to_value(&record_4).unwrap();
    assert_eq!(record_4_json, json!([]));

    for record_4 in [
        serde_json::from_str::<Record4>(&record_4_json.to_string()).unwrap(),
        serde_json::from_value::<Record4>(record_4_json).unwrap(),
    ] {
        let UnpackedRecord4 {} = record_4.unpack();
    }

    println!("serialize_deserialize_json OK");
}

fn serialize_deserialize_bincode() {
    use crate::truc::serialize_deserialize::*;

    let record_0 = Record0::new(UnpackedRecord0 {
        datum_a: 1,
        datum_b: 2,
    });

    let record_0_bincode = bincode::serialize(&record_0).unwrap();

    {
        let record_0 = bincode::deserialize::<Record0>(&record_0_bincode).unwrap();
        assert_eq!(*record_0.datum_a(), 1);
        assert_eq!(*record_0.datum_b(), 2);
        let UnpackedRecord0 {
            datum_a: _,
            datum_b: _,
        } = record_0.unpack();
    }

    let record_1 = Record1::from((record_0, UnpackedRecordIn1 { datum_c: 3 }));

    let record_1_bincode = bincode::serialize(&record_1).unwrap();

    {
        let record_1 = bincode::deserialize::<Record1>(&record_1_bincode).unwrap();
        assert_eq!(*record_1.datum_a(), 1);
        assert_eq!(*record_1.datum_b(), 2);
        assert_eq!(*record_1.datum_c(), 3);
        let UnpackedRecord1 {
            datum_a: _,
            datum_b: _,
            datum_c: _,
        } = record_1.unpack();
    }

    let record_2 = Record2::from((record_1, UnpackedRecordIn2 {}));

    let record_2_bincode = bincode::serialize(&record_2).unwrap();

    {
        let record_2 = bincode::deserialize::<Record2>(&record_2_bincode).unwrap();
        assert_eq!(*record_2.datum_b(), 2);
        assert_eq!(*record_2.datum_c(), 3);
        let UnpackedRecord2 {
            datum_b: _,
            datum_c: _,
        } = record_2.unpack();
    }

    let record_3 = Record3::from((
        record_2,
        UnpackedRecordIn3 {
            datum_v: vec![2, 12, 42],
        },
    ));

    let record_3_bincode = bincode::serialize(&record_3).unwrap();

    {
        let record_3 = bincode::deserialize::<Record3>(&record_3_bincode).unwrap();
        assert_eq!(*record_3.datum_b(), 2);
        assert_eq!(*record_3.datum_c(), 3);
        assert_eq!(*record_3.datum_v(), vec![2, 12, 42]);
        let UnpackedRecord3 {
            datum_b: _,
            datum_c: _,
            datum_v: _,
        } = record_3.unpack();
    }

    let record_4 = Record4::from((record_3, UnpackedRecordIn4 {}));

    let record_4_bincode = bincode::serialize(&record_4).unwrap();

    let record_4 = bincode::deserialize::<Record4>(&record_4_bincode).unwrap();
    let UnpackedRecord4 {} = record_4.unpack();

    println!("serialize_deserialize_bincode OK");
}

fn main() -> Result<(), String> {
    machin();
    serialize_deserialize_json();
    serialize_deserialize_bincode();
    Ok(())
}
