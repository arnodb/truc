use crate::stream::sync::SyncStream;

/// Stream any item.
pub struct ResultOptionVectorStream<T, E> {
    input: Vec<Result<Option<T>, E>>,
}

impl<T, E> ResultOptionVectorStream<T, E> {
    pub fn new(mut input: Vec<Result<Option<T>, E>>) -> Self {
        input.reverse();
        Self { input }
    }
}

impl<T, E> SyncStream for ResultOptionVectorStream<T, E> {
    type Item = T;
    type Error = E;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.input.pop().unwrap_or(Ok(None))
    }
}

#[test]
fn should_stream_result_option_vector() {
    let mut stream =
        ResultOptionVectorStream::new(vec![Ok(Some("a")), Ok(Some("b")), Ok(None), Err("e")]);
    assert_matches!(stream.next(), Ok(Some(v)) if v == "a");
    assert_matches!(stream.next(), Ok(Some(v)) if v == "b");
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Err(e) if e == "e");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}

/// Stream a vector of items without error.
pub struct VectorStream<T> {
    input: Vec<T>,
}

impl<T> VectorStream<T> {
    pub fn new(mut input: Vec<T>) -> Self {
        input.reverse();
        Self { input }
    }
}

impl<T> SyncStream for VectorStream<T> {
    type Item = T;
    type Error = ();

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.input.pop())
    }
}

#[test]
fn should_stream_vector() {
    let mut stream = VectorStream::new(vec!["a", "b"]);
    assert_matches!(stream.next(), Ok(Some(v)) if v == "a");
    assert_matches!(stream.next(), Ok(Some(v)) if v == "b");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
