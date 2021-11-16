#[macro_use]
extern crate static_assertions;

include!(concat!(env!("OUT_DIR"), "/machin_truc.rs"));

pub mod index_first_char {
    include!(concat!(env!("OUT_DIR"), "/index_first_char.rs"));
}
