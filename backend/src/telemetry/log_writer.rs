//! A slow collector must not block HTTP workers or termination signals.
use crate::error::{AppError, ErrorCode};
use prometheus_client::metrics::counter::Counter;
use std::{
    io::{self, Write},
    sync::mpsc::{self, SyncSender},
    time::Duration,
};

const QUEUED_EVENTS: usize = 256;
const MAX_EVENT_BYTES: usize = 16 * 1024;
const FLUSH_BUDGET: Duration = Duration::from_millis(100);

enum Message {
    Event(Vec<u8>),
    Shutdown(SyncSender<()>),
}

#[derive(Clone)]
pub(super) struct QueuedWriter {
    sender: SyncSender<Message>,
    dropped: Counter,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for QueuedWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Write for QueuedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_EVENT_BYTES
            || self
                .sender
                .try_send(Message::Event(bytes.to_vec()))
                .is_err()
        {
            self.dropped.inc();
        }
        // Overflow deliberately drops the complete JSON event. Returning an
        // error could make the formatter fall back to synchronous diagnostics.
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Keep alive until the service stops. Flushes only within a fixed budget;
/// a blocked collector never gets joined or logged to during termination.
#[must_use]
pub struct LoggingGuard {
    sender: SyncSender<Message>,
}

impl Drop for LoggingGuard {
    fn drop(&mut self) {
        let (acknowledge, received) = mpsc::sync_channel(1);
        if self.sender.try_send(Message::Shutdown(acknowledge)).is_ok() {
            let _ = received.recv_timeout(FLUSH_BUDGET);
        }
    }
}

pub(super) fn start(dropped: Counter) -> Result<(QueuedWriter, LoggingGuard), AppError> {
    let (sender, receiver) = mpsc::sync_channel(QUEUED_EVENTS);
    let failures = dropped.clone();
    // The thread owns all blocking I/O. It is intentionally detached: if the
    // sink stalls, process exit reclaims it after the guard's bounded flush.
    std::thread::Builder::new()
        .name("branch-log-writer".into())
        .spawn(move || {
            let mut output = io::stdout();
            for message in receiver {
                match message {
                    Message::Event(bytes) => {
                        if output.write_all(&bytes).is_err() {
                            failures.inc();
                        }
                    }
                    Message::Shutdown(acknowledge) => {
                        let _ = output.flush();
                        let _ = acknowledge.try_send(());
                        break;
                    }
                }
            }
        })
        .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
    Ok((
        QueuedWriter {
            sender: sender.clone(),
            dropped,
        },
        LoggingGuard { sender },
    ))
}
