include!(concat!(env!("OUT_DIR"), "/machin_truc.rs"));

pub mod index_first_char {
    pub mod def_1 {
        include!(concat!(env!("OUT_DIR"), "/index_first_char_1.rs"));
    }

    pub mod def_2 {
        include!(concat!(env!("OUT_DIR"), "/index_first_char_2.rs"));
    }
}

pub mod serialize_deserialize {
    include!(concat!(env!("OUT_DIR"), "/serialize_deserialize.rs"));
}
