//! Runs SimpleFIN network work on its own thread with an Asupersync runtime,
//! so the GPUI main thread never blocks. The result arrives on a oneshot
//! channel that a GPUI task can await.

use std::future::Future;

use asupersync::Cx;
use asupersync::runtime::RuntimeBuilder;
use futures::channel::oneshot;

/// Spawns `work` on a dedicated thread and returns a receiver for its result.
/// A dropped receiver (window closed) simply discards the result.
pub fn spawn_io<T, F, Fut>(name: &str, work: F) -> oneshot::Receiver<anyhow::Result<T>>
where
    T: Send + 'static,
    F: FnOnce(Cx) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let (tx, rx) = oneshot::channel();
    let spawned = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let result = RuntimeBuilder::current_thread()
                .build()
                .map_err(|err| anyhow::anyhow!("failed to start async runtime: {err}"))
                .and_then(|rt| rt.block_on(async { work(Cx::for_request()).await }));
            let _ = tx.send(result);
        });
    if let Err(err) = spawned {
        log::error!("failed to spawn {name} thread: {err}");
    }
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivers_the_result() {
        let rx = spawn_io("test", |_cx| async { Ok(41 + 1) });
        assert_eq!(futures::executor::block_on(rx).unwrap().unwrap(), 42);
    }

    #[test]
    fn delivers_errors() {
        let rx = spawn_io::<(), _, _>("test", |_cx| async { anyhow::bail!("boom") });
        let err = futures::executor::block_on(rx).unwrap().unwrap_err();
        assert_eq!(err.to_string(), "boom");
    }
}
