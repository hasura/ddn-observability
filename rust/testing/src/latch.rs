use std::future;
use std::sync::{Arc, Mutex};
use std::task;

/// A latch is a future that is declared pending until "unlocked".
///
/// Typically, the creator will know how to unlock the latch, but it can be
/// handed to another object as an opaque future which can only be `await`-ed
/// upon.
#[derive(Clone)]
pub struct Latch(Arc<Mutex<LatchState>>);

impl Latch {
    /// Creates a new latch in the "locked" state.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(LatchState {
            locked: true,
            waker: None,
        })))
    }

    /// Unlocks the latch, waking any tasks awaiting it.
    ///
    /// This operation is irreversible.
    pub fn unlock(&self) {
        let mut inner = self.0.lock().unwrap();
        inner.locked = false;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

struct LatchState {
    locked: bool,
    waker: Option<task::Waker>,
}

impl future::Future for Latch {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let mut inner = self.0.lock().unwrap();
        if inner.locked {
            inner.waker = Some(cx.waker().clone());
            task::Poll::Pending
        } else {
            task::Poll::Ready(())
        }
    }
}
