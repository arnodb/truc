#[macro_use]
extern crate assert_matches;

use streamink::{stream::sync::SyncStream, vector::ResultOptionVectorStream};

#[test]
fn should_stream_map_err() {
    let mut stream = ResultOptionVectorStream::new(vec![Ok(Some("a")), Err("e")])
        .map_err(|err| err.chars().next());
    assert_matches!(stream.next(), Ok(Some(v)) if v == "a");
    assert_matches!(stream.next(), Err(Some('e')));
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
