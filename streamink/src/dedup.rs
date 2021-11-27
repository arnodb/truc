use crate::stream::sync::SyncStream;

/// Dedups items in memory and stream them.
#[derive(new)]
pub struct Dedup<I: SyncStream<Item = R, Error = E>, R, E, C>
where
    C: Fn(&R, &R) -> bool,
{
    input: I,
    eq: C,
    #[new(default)]
    buffer: Option<R>,
    #[new(default)]
    end_of_input: bool,
}

impl<I: SyncStream<Item = R, Error = E>, R, E, C> SyncStream for Dedup<I, R, E, C>
where
    C: Fn(&R, &R) -> bool,
{
    type Item = R;
    type Error = E;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if !self.end_of_input {
            while let Some(rec) = self.input.next()? {
                if let Some(buffer) = &self.buffer {
                    if !(self.eq)(buffer, &rec) {
                        let ret = self.buffer.replace(rec);
                        return Ok(ret);
                    }
                } else {
                    self.buffer = Some(rec);
                }
            }
            self.end_of_input = true;
        }
        Ok(self.buffer.take())
    }
}

#[test]
fn should_dedup_stream() {
    use crate::vector::VectorStream;

    let mut stream = Dedup::new(
        VectorStream::new(vec![("a", 12), ("a", 12), ("a", 42), ("b", 42)]),
        |a, b| (&a.0).eq(&b.0) && (&a.1).eq(&b.1),
    );
    assert_matches!(stream.next(), Ok(Some((v, 12))) if v == "a");
    assert_matches!(stream.next(), Ok(Some((v, 42))) if v == "a");
    assert_matches!(stream.next(), Ok(Some((v, 42))) if v == "b");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
