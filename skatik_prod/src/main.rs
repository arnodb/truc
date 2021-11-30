#[macro_use]
extern crate static_assertions;

#[allow(dead_code)]
#[allow(clippy::borrowed_box)]
mod chain;

fn main() {
    chain::main().unwrap();
}
