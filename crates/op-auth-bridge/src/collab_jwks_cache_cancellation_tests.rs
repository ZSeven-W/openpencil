#![cfg(test)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::SigningKey;
use serde_json::json;

use super::*;

struct CancelThenSucceedFetcher {
    attempts: AtomicUsize,
    started: mpsc::SyncSender<()>,
    body: Vec<u8>,
}

impl CancelThenSucceedFetcher {
    fn success(&self) -> CollabJwksFetchResponse {
        CollabJwksFetchResponse::Modified {
            body: self.body.clone(),
            etag: Some("\"after-cancel\"".to_owned()),
            max_age_seconds: 60,
        }
    }
}

impl CollabJwksFetcher for CancelThenSucceedFetcher {
    fn fetch(
        &self,
        _request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Ok(self.success())
    }

    fn fetch_cancellable(
        &self,
        _request: CollabJwksFetchRequest<'_>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt > 0 {
            return Ok(self.success());
        }
        self.started.send(()).unwrap();
        while !cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        Err(CollabJwksFetchError::Cancelled)
    }
}

struct GateFetcher {
    started: mpsc::SyncSender<()>,
    release: Arc<AtomicBool>,
    body: Vec<u8>,
}

impl CollabJwksFetcher for GateFetcher {
    fn fetch(
        &self,
        _request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        self.started.send(()).unwrap();
        while !self.release.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(CollabJwksFetchResponse::Modified {
            body: self.body.clone(),
            etag: Some("\"gate\"".to_owned()),
            max_age_seconds: 60,
        })
    }
}

fn key(seed: u8) -> [u8; 32] {
    SigningKey::from_bytes(&[seed; 32])
        .verifying_key()
        .to_bytes()
}

fn jwks(keys: &[(&str, u8)]) -> Vec<u8> {
    let keys = keys
        .iter()
        .map(|(key_id, seed)| {
            json!({
                "kty": "OKP",
                "crv": "Ed25519",
                "alg": "Ed25519",
                "use": "sig",
                "key_ops": ["verify"],
                "kid": key_id,
                "x": URL_SAFE_NO_PAD.encode(key(*seed)),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&json!({ "keys": keys })).unwrap()
}

#[test]
fn cancellation_during_refresh_does_not_throttle_immediate_restart() {
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let cache = Arc::new(
        CollabJwksCache::new(
            "https://issuer.example/jwks",
            CancelThenSucceedFetcher {
                attempts: AtomicUsize::new(0),
                started: started_sender,
                body: jwks(&[("key_A", 1)]),
            },
            CollabJwksCacheLimits::default(),
        )
        .unwrap(),
    );
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cache = Arc::clone(&cache);
    let worker_cancelled = Arc::clone(&cancelled);
    let now = Instant::now();
    let (done_sender, done_receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = worker_cache.verification_key_cancellable("key_A", now, &|| {
            worker_cancelled.load(Ordering::Acquire)
        });
        done_sender.send(result).unwrap();
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("refresh must start");
    let cancellation_started = Instant::now();
    cancelled.store(true, Ordering::Release);
    assert_eq!(
        done_receiver.recv_timeout(Duration::from_millis(500)),
        Ok(Err(CollabJwksError::Fetch(CollabJwksFetchError::Cancelled)))
    );
    worker.join().unwrap();
    assert!(cancellation_started.elapsed() < Duration::from_secs(1));

    assert_eq!(
        cache.verification_key("key_A", now).unwrap(),
        key(1),
        "a cancelled attempt must not leave refresh backoff behind"
    );
}

#[test]
fn cancellation_interrupts_waiting_for_the_cache_mutex() {
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let release = Arc::new(AtomicBool::new(false));
    let cache = Arc::new(
        CollabJwksCache::new(
            "https://issuer.example/jwks",
            GateFetcher {
                started: started_sender,
                release: Arc::clone(&release),
                body: jwks(&[("key_A", 1)]),
            },
            CollabJwksCacheLimits::default(),
        )
        .unwrap(),
    );
    let first_cache = Arc::clone(&cache);
    let now = Instant::now();
    let first = std::thread::spawn(move || first_cache.verification_key("key_A", now));
    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("first refresh must hold the cache mutex");

    let cancelled = Arc::new(AtomicBool::new(false));
    let second_cache = Arc::clone(&cache);
    let second_cancelled = Arc::clone(&cancelled);
    let cancellation_checks = Arc::new(AtomicUsize::new(0));
    let second_checks = Arc::clone(&cancellation_checks);
    let (waiting_sender, waiting_receiver) = mpsc::sync_channel(1);
    let (done_sender, done_receiver) = mpsc::sync_channel(1);
    let second = std::thread::spawn(move || {
        let result = second_cache.verification_key_cancellable("key_A", now, &|| {
            if second_checks.fetch_add(1, Ordering::SeqCst) == 2 {
                waiting_sender.send(()).unwrap();
            }
            second_cancelled.load(Ordering::Acquire)
        });
        done_sender.send(result).unwrap();
    });
    waiting_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("second lookup must observe the mutex as locked");
    let cancellation_started = Instant::now();
    cancelled.store(true, Ordering::Release);
    let second_result = done_receiver.recv_timeout(Duration::from_millis(500));
    release.store(true, Ordering::Release);

    assert_eq!(
        second_result,
        Ok(Err(CollabJwksError::Fetch(CollabJwksFetchError::Cancelled)))
    );
    second.join().unwrap();
    assert_eq!(first.join().unwrap().unwrap(), key(1));
    assert!(cancellation_started.elapsed() < Duration::from_secs(1));
}
