//! Kehyksen löytäminen simuloidusta sarjavirrasta ilman fyysistä lukijaa.
use as2015_rs::{emit, reader::FrameScanner};

fn sample() -> &'static [u8] {
    include_bytes!("../fixtures/synthetic/emit-frame.raw")
}

#[test]
fn finds_frame_after_noise_even_when_parts_are_split() {
    let mut scanner = FrameScanner::new();
    assert!(scanner.push(&[0x7d, 0xef, 0x20]).is_none());
    assert!(scanner.push(&sample()[1..100]).is_none());
    let frame = scanner.push(&sample()[100..]).unwrap();
    assert_eq!(frame.card_number, 123456);
    assert!(frame.records.iter().any(|punch| punch.code == 250));
}

#[test]
fn finds_next_frame_after_partial_and_invalid_frames() {
    let mut scanner = FrameScanner::new();
    let mut wrong = sample().to_vec();
    wrong[2] = emit::XOR_MASK; // Korttinumeron pienin tavu muuttuu.
    let mut stream = vec![0x00; 17];
    stream.extend_from_slice(&wrong[..80]);
    stream.extend_from_slice(sample());
    // Lukeminen alkaa kesken kortin ja jatkuu seuraavasta eheästä kehyksestä.
    let frame = scanner.push(&stream).unwrap();
    assert_eq!(frame.card_number, 123456);
}

#[test]
fn no_frame_is_reported_for_noise() {
    let mut scanner = FrameScanner::new();
    assert!(scanner.push(&vec![0xdf; 1500]).is_none());
    assert!(scanner.push(&[0x20]).is_none());
}
