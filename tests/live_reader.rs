//! Taustalukijan elinkaari ja rajattu tapahtumajono ilman fyysistä laitetta.
use as2026::{
    emit,
    live_reader::{EVENT_QUEUE_CAPACITY, ReaderEvent, ReaderWorker},
};
use std::{
    collections::VecDeque,
    io::{self, Read},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::Receiver,
    },
    thread,
    time::{Duration, Instant},
};

fn sample() -> &'static [u8] {
    include_bytes!("../fixtures/synthetic/emit-frame.raw")
}

fn receive(receiver: &Receiver<ReaderEvent>) -> ReaderEvent {
    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("lukijan tapahtuma jäi saamatta")
}

struct ScriptedPort {
    reads: VecDeque<io::Result<Vec<u8>>>,
    drop_count: Arc<AtomicUsize>,
}

impl Read for ScriptedPort {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let Some(read) = self.reads.pop_front() else {
            return Err(io::ErrorKind::TimedOut.into());
        };
        let bytes = read?;
        assert!(bytes.len() <= buffer.len());
        buffer[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

impl Drop for ScriptedPort {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn keeps_one_reader_for_fragmented_repeated_and_different_cards() {
    let mut second = sample().to_vec();
    second[2..5].copy_from_slice(&[42 ^ emit::XOR_MASK, emit::XOR_MASK, emit::XOR_MASK]);
    let drops = Arc::new(AtomicUsize::new(0));
    let (mut worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        ScriptedPort {
            reads: VecDeque::from([
                Ok(sample()[..100].to_vec()),
                Err(io::ErrorKind::TimedOut.into()),
                Ok([&sample()[100..], sample()].concat()),
                Ok(second),
            ]),
            drop_count: drops.clone(),
        },
    )
    .unwrap();
    assert!(matches!(receive(&receiver), ReaderEvent::Connected { port } if port == "Testiportti"));
    for expected in [123456, 123456, 42] {
        let ReaderEvent::Frame { frame, read_clock } = receive(&receiver) else {
            panic!("odotettiin luettua korttia");
        };
        assert_eq!(frame.card_number, expected);
        assert!(read_clock.seconds_since_midnight().is_ok());
    }
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(worker.stop().unwrap().is_empty());
    assert!(matches!(receive(&receiver), ReaderEvent::Stopped));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(worker.stop().unwrap().is_empty());
}

#[test]
fn reports_read_error_and_stops_without_reopening_the_port() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (mut worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        ScriptedPort {
            reads: VecDeque::from([Err(io::ErrorKind::BrokenPipe.into())]),
            drop_count: drops.clone(),
        },
    )
    .unwrap();
    assert!(matches!(receive(&receiver), ReaderEvent::Connected { .. }));
    assert!(
        matches!(receive(&receiver), ReaderEvent::Error(message) if message.starts_with("Sarjaportin lukeminen epäonnistui:"))
    );
    assert!(matches!(receive(&receiver), ReaderEvent::Stopped));
    assert!(worker.stop().unwrap().is_empty());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct RepeatingPort {
    reads: Arc<AtomicUsize>,
}

impl Read for RepeatingPort {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        buffer[..sample().len()].copy_from_slice(sample());
        Ok(sample().len())
    }
}

#[test]
fn backpressure_bounds_memory_and_stop_returns_the_unsent_frame() {
    let reads = Arc::new(AtomicUsize::new(0));
    let (mut worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        RepeatingPort {
            reads: reads.clone(),
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while reads.load(Ordering::SeqCst) < EVENT_QUEUE_CAPACITY {
        assert!(Instant::now() < deadline, "jono ei täyttynyt");
        thread::sleep(Duration::from_millis(1));
    }
    // Vastaanottaja ei käsittele tapahtumia: lisälukuja ei saa kertyä rajatta.
    thread::sleep(Duration::from_millis(30));
    assert_eq!(reads.load(Ordering::SeqCst), EVENT_QUEUE_CAPACITY);
    let started = Instant::now();
    let pending = worker.stop().unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    let queued: Vec<_> = receiver.try_iter().collect();
    assert_eq!(queued.len(), EVENT_QUEUE_CAPACITY);
    assert!(matches!(queued[0], ReaderEvent::Connected { .. }));
    assert_eq!(
        queued
            .iter()
            .filter(|event| matches!(event, ReaderEvent::Frame { .. }))
            .count(),
        EVENT_QUEUE_CAPACITY - 1
    );
    assert_eq!(pending.len(), 2);
    assert!(matches!(pending[0], ReaderEvent::Frame { .. }));
    assert!(matches!(pending[1], ReaderEvent::Stopped));
}

struct BatchPort {
    next_card: u32,
    frames_read: Arc<AtomicUsize>,
}

impl Read for BatchPort {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        // Ensimmäinen luku kohdistaa täyden jonon seuraavan kahden kehyksen erän alkuun.
        let count = if self.next_card == 1 { 1 } else { 2 };
        for index in 0..count {
            let mut frame = sample().to_vec();
            let card = self.next_card.to_le_bytes();
            for (byte, value) in frame[2..5].iter_mut().zip(card) {
                *byte = value ^ emit::XOR_MASK;
            }
            let start = index * frame.len();
            buffer[start..start + frame.len()].copy_from_slice(&frame);
            self.next_card += 1;
        }
        self.frames_read.fetch_add(count, Ordering::SeqCst);
        Ok(count * sample().len())
    }
}

#[test]
fn stopping_retains_already_buffered_complete_frames_in_order() {
    let frames_read = Arc::new(AtomicUsize::new(0));
    let (mut worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        BatchPort {
            next_card: 1,
            frames_read: frames_read.clone(),
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while frames_read.load(Ordering::SeqCst) < EVENT_QUEUE_CAPACITY + 1 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    let pending = worker.stop().unwrap();
    assert_eq!(pending.len(), 3);
    let all: Vec<_> = receiver.try_iter().chain(pending).collect();
    let cards: Vec<_> = all
        .iter()
        .filter_map(|event| match event {
            ReaderEvent::Frame { frame, .. } => Some(frame.card_number),
            _ => None,
        })
        .collect();
    assert_eq!(
        cards,
        (1..=EVENT_QUEUE_CAPACITY as u32 + 1).collect::<Vec<_>>()
    );
    assert!(matches!(all.last(), Some(ReaderEvent::Stopped)));
}

#[test]
fn dropping_worker_closes_idle_reader_without_an_explicit_stop() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        ScriptedPort {
            reads: VecDeque::new(),
            drop_count: drops.clone(),
        },
    )
    .unwrap();
    assert!(matches!(receive(&receiver), ReaderEvent::Connected { .. }));
    drop(worker);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(matches!(receive(&receiver), ReaderEvent::Stopped));
}

#[test]
fn disconnected_receiver_returns_undelivered_data_and_closes_reader() {
    let reads = Arc::new(AtomicUsize::new(0));
    let (mut worker, receiver) = ReaderWorker::from_reader(
        "Testiportti".into(),
        RepeatingPort {
            reads: reads.clone(),
        },
    )
    .unwrap();
    assert!(matches!(receive(&receiver), ReaderEvent::Connected { .. }));
    let deadline = Instant::now() + Duration::from_secs(2);
    while reads.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    drop(receiver);
    // Täysi tai katkennut kanava ei saa jättää säiettä pysäytyksessä odottamaan.
    assert!(!worker.stop().unwrap().is_empty());
}
