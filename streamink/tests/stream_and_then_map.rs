#[macro_use]
extern crate assert_matches;

use streamink::{stream::sync::SyncStream, vector::ResultOptionVectorStream};

#[test]
fn should_stream_and_then_map() {
    let mut stream = ResultOptionVectorStream::new(vec![Ok(Some("a")), Err("e")])
        .and_then_map(|item| Ok(format!("Item: {}", item)));
    assert_matches!(stream.next(), Ok(Some(v)) if v == "Item: a");
    assert_matches!(stream.next(), Err(err) if err == "e");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
