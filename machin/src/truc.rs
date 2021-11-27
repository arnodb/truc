include!(concat!(env!("OUT_DIR"), "/machin_truc.rs"));

pub mod index_first_char {
    pub mod def_1 {
        include!(concat!(env!("OUT_DIR"), "/index_first_char_1.rs"));
    }

    pub mod def_2 {
        include!(concat!(env!("OUT_DIR"), "/index_first_char_2.rs"));
    }
}
