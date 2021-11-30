pub mod streams {
    include!(concat!(env!("OUT_DIR"), "/chain_streams.rs"));
}

include!(concat!(env!("OUT_DIR"), "/chain.rs"));
