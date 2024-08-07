use std::future::Future;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::sync::watch as tokio_watch;

use crate::async_runtime::mpsc_unbounded;
use crate::async_runtime::mpsc_unbounded::MpscUnbounded;
use crate::async_runtime::watch;
use crate::type_config::OneshotSender;
use crate::AsyncRuntime;
use crate::OptionalSend;
use crate::OptionalSync;
use crate::TokioInstant;

/// `Tokio` is the default asynchronous executor.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TokioRuntime;

impl AsyncRuntime for TokioRuntime {
    type JoinError = tokio::task::JoinError;
    type JoinHandle<T: OptionalSend + 'static> = tokio::task::JoinHandle<T>;
    type Sleep = tokio::time::Sleep;
    type Instant = TokioInstant;
    type TimeoutError = tokio::time::error::Elapsed;
    type Timeout<R, T: Future<Output = R> + OptionalSend> = tokio::time::Timeout<T>;
    type ThreadLocalRng = rand::rngs::ThreadRng;
    type OneshotSender<T: OptionalSend> = tokio::sync::oneshot::Sender<T>;
    type OneshotReceiver<T: OptionalSend> = tokio::sync::oneshot::Receiver<T>;
    type OneshotReceiverError = tokio::sync::oneshot::error::RecvError;

    #[inline]
    fn spawn<T>(future: T) -> Self::JoinHandle<T::Output>
    where
        T: Future + OptionalSend + 'static,
        T::Output: OptionalSend + 'static,
    {
        #[cfg(feature = "singlethreaded")]
        {
            tokio::task::spawn_local(future)
        }
        #[cfg(not(feature = "singlethreaded"))]
        {
            tokio::task::spawn(future)
        }
    }

    #[inline]
    fn sleep(duration: Duration) -> Self::Sleep {
        tokio::time::sleep(duration)
    }

    #[inline]
    fn sleep_until(deadline: Self::Instant) -> Self::Sleep {
        tokio::time::sleep_until(deadline)
    }

    #[inline]
    fn timeout<R, F: Future<Output = R> + OptionalSend>(duration: Duration, future: F) -> Self::Timeout<R, F> {
        tokio::time::timeout(duration, future)
    }

    #[inline]
    fn timeout_at<R, F: Future<Output = R> + OptionalSend>(deadline: Self::Instant, future: F) -> Self::Timeout<R, F> {
        tokio::time::timeout_at(deadline, future)
    }

    #[inline]
    fn is_panic(join_error: &Self::JoinError) -> bool {
        join_error.is_panic()
    }

    #[inline]
    fn thread_rng() -> Self::ThreadLocalRng {
        rand::thread_rng()
    }

    #[inline]
    fn oneshot<T>() -> (Self::OneshotSender<T>, Self::OneshotReceiver<T>)
    where T: OptionalSend {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (tx, rx)
    }

    type MpscUnbounded = TokioMpscUnbounded;
    type Watch = TokioWatch;
}

impl<T> OneshotSender<T> for tokio::sync::oneshot::Sender<T> {
    #[inline]
    fn send(self, t: T) -> Result<(), T> {
        self.send(t)
    }
}

pub struct TokioMpscUnbounded;

impl MpscUnbounded for TokioMpscUnbounded {
    type Sender<T: OptionalSend> = mpsc::UnboundedSender<T>;
    type Receiver<T: OptionalSend> = mpsc::UnboundedReceiver<T>;
    type WeakSender<T: OptionalSend> = mpsc::WeakUnboundedSender<T>;

    /// Creates an unbounded mpsc channel for communicating between asynchronous
    /// tasks without backpressure.
    fn channel<T: OptionalSend>() -> (Self::Sender<T>, Self::Receiver<T>) {
        mpsc::unbounded_channel()
    }
}

impl<T> mpsc_unbounded::MpscUnboundedSender<TokioMpscUnbounded, T> for mpsc::UnboundedSender<T>
where T: OptionalSend
{
    #[inline]
    fn send(&self, msg: T) -> Result<(), mpsc_unbounded::SendError<T>> {
        self.send(msg).map_err(|e| mpsc_unbounded::SendError(e.0))
    }

    #[inline]
    fn downgrade(&self) -> <TokioMpscUnbounded as MpscUnbounded>::WeakSender<T> {
        self.downgrade()
    }
}

impl<T> mpsc_unbounded::MpscUnboundedReceiver<T> for mpsc::UnboundedReceiver<T>
where T: OptionalSend
{
    #[inline]
    async fn recv(&mut self) -> Option<T> {
        self.recv().await
    }

    #[inline]
    fn try_recv(&mut self) -> Result<T, mpsc_unbounded::TryRecvError> {
        self.try_recv().map_err(|e| match e {
            mpsc::error::TryRecvError::Empty => mpsc_unbounded::TryRecvError::Empty,
            mpsc::error::TryRecvError::Disconnected => mpsc_unbounded::TryRecvError::Disconnected,
        })
    }
}

impl<T> mpsc_unbounded::MpscUnboundedWeakSender<TokioMpscUnbounded, T> for mpsc::WeakUnboundedSender<T>
where T: OptionalSend
{
    #[inline]
    fn upgrade(&self) -> Option<<TokioMpscUnbounded as MpscUnbounded>::Sender<T>> {
        self.upgrade()
    }
}

pub struct TokioWatch;

impl watch::Watch for TokioWatch {
    type Sender<T: OptionalSend + OptionalSync> = tokio_watch::Sender<T>;
    type Receiver<T: OptionalSend + OptionalSync> = tokio_watch::Receiver<T>;

    type Ref<'a, T: OptionalSend + 'a> = tokio_watch::Ref<'a, T>;

    fn channel<T: OptionalSend + OptionalSync>(init: T) -> (Self::Sender<T>, Self::Receiver<T>) {
        tokio_watch::channel(init)
    }
}

impl<T> watch::WatchSender<TokioWatch, T> for tokio_watch::Sender<T>
where T: OptionalSend + OptionalSync
{
    fn send(&self, value: T) -> Result<(), watch::SendError<T>> {
        self.send(value).map_err(|e| watch::SendError(e.0))
    }

    fn send_if_modified<F>(&self, modify: F) -> bool
    where F: FnOnce(&mut T) -> bool {
        self.send_if_modified(modify)
    }

    fn borrow_watched(&self) -> <TokioWatch as watch::Watch>::Ref<'_, T> {
        self.borrow()
    }
}

impl<T> watch::WatchReceiver<TokioWatch, T> for tokio_watch::Receiver<T>
where T: OptionalSend + OptionalSync
{
    async fn changed(&mut self) -> Result<(), watch::RecvError> {
        self.changed().await.map_err(|_| watch::RecvError(()))
    }

    fn borrow_watched(&self) -> <TokioWatch as watch::Watch>::Ref<'_, T> {
        self.borrow()
    }
}
