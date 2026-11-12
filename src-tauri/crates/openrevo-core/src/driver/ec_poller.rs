use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

pub(super) struct EcPoller {
    stopped: Arc<AtomicBool>,
    thread: Thread,
}

impl EcPoller {
    pub(super) fn spawn(
        initialize: impl FnOnce() -> bool + Send + 'static,
        mut poll: impl FnMut(&AtomicBool) -> Duration + Send + 'static,
    ) -> Self {
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stop = stopped.clone();
        let worker = thread::spawn(move || {
            if !wait(&worker_stop, Duration::from_millis(500)) || !initialize() {
                return;
            }
            while !worker_stop.load(Ordering::Acquire) {
                if !wait(&worker_stop, poll(&worker_stop)) {
                    return;
                }
            }
        });
        Self {
            stopped,
            thread: worker.thread().clone(),
        }
    }
}

impl Drop for EcPoller {
    fn drop(&mut self) {
        // shortcut: Active driver calls finish; isolate I/O when hard cancellation is required.
        self.stopped.store(true, Ordering::Release);
        self.thread.unpark();
    }
}

fn wait(stopped: &AtomicBool, duration: Duration) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        if stopped.load(Ordering::Acquire) {
            return false;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return true;
        }
        thread::park_timeout(remaining);
    }
}

pub(super) fn set_bits_if_running(stopped: &AtomicBool, current: u8, bits: u8) -> u8 {
    if stopped.load(Ordering::Acquire) {
        current
    } else {
        current | bits
    }
}

#[cfg(test)]
mod tests {
    use super::super::EcBridge;
    use super::*;
    use std::sync::atomic::{AtomicU8, AtomicUsize};
    use std::sync::{mpsc, Weak};

    fn released<T>(cache: &Weak<T>) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while cache.strong_count() != 0 {
            assert!(
                Instant::now() < deadline,
                "Stopped poller retained its cache"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn regression_dropped_bridge_cancels_pending_initialization_and_releases_cache() {
        let mut bridge = EcBridge::dummy();
        let cache = bridge.cache.clone();
        let weak = Arc::downgrade(&cache);
        let calls = Arc::new(AtomicUsize::new(0));
        let initialized = calls.clone();
        let polled = calls.clone();
        bridge.poller = Some(EcPoller::spawn(
            move || {
                initialized.fetch_add(1, Ordering::SeqCst);
                true
            },
            move |_| {
                let _ = &cache;
                polled.fetch_add(1, Ordering::SeqCst);
                Duration::from_secs(60)
            },
        ));
        drop(bridge);
        released(&weak);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn regression_stop_during_read_skips_pending_write_and_repeated_polling() {
        let mut bridge = EcBridge::dummy();
        let cache = bridge.cache.clone();
        let weak = Arc::downgrade(&cache);
        let value = Arc::new(AtomicU8::new(0));
        let observed = value.clone();
        let polls = Arc::new(AtomicUsize::new(0));
        let counted = polls.clone();
        let (started, ready) = mpsc::channel();
        let (resume, blocked) = mpsc::channel();
        let (finished, completed) = mpsc::channel();
        bridge.poller = Some(EcPoller::spawn(
            || true,
            move |stopped| {
                let _ = &cache;
                counted.fetch_add(1, Ordering::SeqCst);
                started.send(()).unwrap();
                blocked.recv_timeout(Duration::from_secs(2)).unwrap();
                observed.store(
                    set_bits_if_running(stopped, 0, crate::hal::vault::power::AP_EXIST_BIT),
                    Ordering::SeqCst,
                );
                finished.send(()).unwrap();
                Duration::from_secs(60)
            },
        ));
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(bridge);
        resume.send(()).unwrap();
        completed.recv_timeout(Duration::from_secs(2)).unwrap();
        released(&weak);
        assert_eq!(value.load(Ordering::SeqCst), 0);
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn regression_running_bit_updates_preserve_all_unrelated_bits() {
        let bit = crate::hal::vault::power::AP_EXIST_BIT;
        let stopped = AtomicBool::new(false);
        assert_eq!(set_bits_if_running(&stopped, u8::MAX ^ bit, bit), u8::MAX);
        assert_eq!(set_bits_if_running(&stopped, u8::MAX, bit), u8::MAX);
        stopped.store(true, Ordering::Release);
        assert_eq!(
            set_bits_if_running(&stopped, u8::MAX ^ bit, bit),
            u8::MAX ^ bit
        );
    }

    #[test]
    fn regression_explicit_stop_preserves_cache_and_does_not_stop_another_bridge() {
        let mut old = EcBridge::dummy();
        let mut current = EcBridge::dummy();
        let (started, ready) = mpsc::channel();
        for bridge in [&mut old, &mut current] {
            let cache = bridge.cache.clone();
            let started = started.clone();
            bridge.poller = Some(EcPoller::spawn(
                || true,
                move |_| {
                    let _ = &cache;
                    started.send(()).unwrap();
                    Duration::from_secs(60)
                },
            ));
        }
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        old.stop_polling();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Arc::strong_count(&old.cache) != 1 {
            assert!(
                Instant::now() < deadline,
                "Explicit stop did not release the worker cache"
            );
            thread::sleep(Duration::from_millis(1));
        }
        assert!(old.poller.is_none());
        assert_eq!(old.get_battery_limit(), 80);
        assert!(!current
            .poller
            .as_ref()
            .unwrap()
            .stopped
            .load(Ordering::Acquire));
        let weak = Arc::downgrade(&current.cache);
        drop(current);
        released(&weak);
    }

    #[test]
    fn regression_early_unpark_keeps_the_original_poll_interval() {
        let (started, ready) = mpsc::channel();
        let worker = thread::spawn(move || {
            let began = Instant::now();
            started.send(()).unwrap();
            assert!(wait(&AtomicBool::new(false), Duration::from_millis(100)));
            assert!(began.elapsed() >= Duration::from_millis(100));
        });
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.thread().unpark();
        worker.join().unwrap();
    }
}
