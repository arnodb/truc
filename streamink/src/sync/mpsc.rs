use crate::stream::sync::SyncStream;
use std::sync::mpsc::{Receiver, RecvError};

/// Receives records from a `Receiver`
#[derive(new)]
pub struct Receive<R, E> {
    rx: Receiver<Option<R>>,
    #[new(default)]
    _e: std::marker::PhantomData<E>,
    #[new(default)]
    end_of_input: bool,
}

impl<R, E> SyncStream for Receive<R, E>
where
    E: From<RecvError>,
{
    type Item = R;
    type Error = E;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if !self.end_of_input {
            let res = self.rx.recv()?;
            if res.is_none() {
                self.end_of_input = true
            };
            Ok(res)
        } else {
            Ok(None)
        }
    }
}

#[test]
fn should_stream_received_records() {
    use std::sync::mpsc::channel;

    #[derive(Debug)]
    struct Error(String);

    impl From<RecvError> for Error {
        fn from(_: RecvError) -> Self {
            Self("Receive error".to_string())
        }
    }

    let (tx, rx) = channel();
    let mut stream = Receive::<_, Error>::new(rx);
    for i in 0..42 {
        tx.send(Some(i)).unwrap();
    }
    tx.send(None).unwrap();
    for i in 0..42 {
        assert_matches!(stream.next(), Ok(Some(j)) if j == i);
    }
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}

#[test]
fn should_stream_received_records_from_sync_channel() {
    use std::sync::mpsc::sync_channel;

    #[derive(Debug)]
    struct Error(String);

    impl From<RecvError> for Error {
        fn from(_: RecvError) -> Self {
            Self("Receive error".to_string())
        }
    }

    let (tx, rx) = sync_channel(100);
    let mut stream = Receive::<_, Error>::new(rx);
    for i in 0..42 {
        tx.send(Some(i)).unwrap();
    }
    tx.send(None).unwrap();
    for i in 0..42 {
        assert_matches!(stream.next(), Ok(Some(j)) if j == i);
    }
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
