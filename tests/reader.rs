//! Kehyksen löytäminen simuloidusta sarjavirrasta ilman fyysistä lukijaa.
use as2026::{
    emit,
    reader::{CardReader, FrameScanner, port_name},
};
use std::{
    cell::Cell,
    collections::VecDeque,
    io::{self, Read},
    rc::Rc,
};

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
    wrong[2..5].fill(emit::XOR_MASK); // Korttinumero nolla hylätään.
    let mut stream = vec![0x00; 17];
    stream.extend_from_slice(&wrong);
    stream.extend_from_slice(&sample()[..14]); // Katkos ennen lukumerkkiä.
    stream.extend_from_slice(sample());
    // Lukeminen alkaa kesken kortin ja jatkuu seuraavasta eheästä kehyksestä.
    let frame = scanner.push(&stream).unwrap();
    assert_eq!(frame.raw.as_slice(), sample());
}

#[test]
fn no_frame_is_reported_for_noise() {
    let mut scanner = FrameScanner::new();
    assert!(scanner.push(&vec![0xdf; 1500]).is_none());
    assert!(scanner.push(&[0x20]).is_none());
}

#[test]
fn synchronization_bytes_inside_payload_do_not_cut_a_complete_frame() {
    let mut raw = sample().to_vec();
    // Aika 65535 tuottaa kaksi peräkkäistä 0x20-tavua XOR-käsittelyn jälkeen.
    raw[14..16].fill(0x20);
    let mut scanner = FrameScanner::new();
    let frame = scanner.push(&raw).unwrap();
    assert_eq!(frame.raw.as_slice(), raw);
    assert_eq!(frame.records[1].elapsed_seconds, 65535);
}

#[test]
fn every_possible_chunk_boundary_retains_a_single_frame() {
    for split in 0..sample().len() {
        let mut scanner = FrameScanner::new();
        assert!(scanner.push(&sample()[..split]).is_none());
        assert_eq!(
            scanner.push(&sample()[split..]).unwrap().raw.as_slice(),
            sample()
        );
        assert!(scanner.next_frame().is_none());
    }
}

#[test]
fn drains_multiple_different_cards_from_one_buffer_in_order() {
    let mut other = sample().to_vec();
    other[2..5].copy_from_slice(&[42 ^ emit::XOR_MASK, emit::XOR_MASK, emit::XOR_MASK]);
    let mut scanner = FrameScanner::new();
    let stream = [sample(), other.as_slice(), sample()].concat();
    assert_eq!(scanner.push(&stream).unwrap().card_number, 123456);
    assert_eq!(scanner.next_frame().unwrap().card_number, 42);
    assert_eq!(scanner.next_frame().unwrap().card_number, 123456);
    assert!(scanner.next_frame().is_none());
}

#[test]
fn malformed_marker_does_not_prevent_the_next_card() {
    let mut invalid = sample().to_vec();
    invalid[10] = 250 ^ emit::XOR_MASK; // Ensimmäinen merkki ei saa olla lukumerkki.
    let mut scanner = FrameScanner::new();
    assert!(scanner.push(&invalid).is_none());
    assert_eq!(scanner.push(sample()).unwrap().card_number, 123456);
}

struct SimulatedPort {
    reads: VecDeque<io::Result<Vec<u8>>>,
    count: Rc<Cell<usize>>,
}

impl Read for SimulatedPort {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.count.set(self.count.get() + 1);
        let bytes = self.reads.pop_front().expect("odottamaton laiteluku")?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

#[test]
fn persistent_reader_keeps_partial_data_across_timeouts_and_drains_without_rereading() {
    let count = Rc::new(Cell::new(0));
    let port = SimulatedPort {
        reads: VecDeque::from([
            Ok(sample()[..100].to_vec()),
            Err(io::ErrorKind::TimedOut.into()),
            Err(io::ErrorKind::Interrupted.into()),
            Ok([&sample()[100..], sample()].concat()),
        ]),
        count: count.clone(),
    };
    let mut reader = CardReader::from_reader(port);
    for _ in 0..3 {
        assert!(reader.poll().unwrap().is_none());
    }
    assert_eq!(reader.poll().unwrap().unwrap().card_number, 123456);
    assert_eq!(reader.poll().unwrap().unwrap().card_number, 123456);
    assert_eq!(count.get(), 4);
}

#[test]
fn persistent_reader_reports_real_io_errors_in_finnish() {
    let mut reader = CardReader::from_reader(SimulatedPort {
        reads: VecDeque::from([Err(io::ErrorKind::BrokenPipe.into())]),
        count: Rc::new(Cell::new(0)),
    });
    assert!(
        reader
            .poll()
            .unwrap_err()
            .starts_with("Sarjaportin lukeminen epäonnistui:")
    );
}

#[test]
fn explicit_linux_and_windows_port_names_are_preserved() {
    assert_eq!(port_name(Some(" /dev/ttyUSB0 ")).unwrap(), "/dev/ttyUSB0");
    assert_eq!(port_name(Some("COM12")).unwrap(), "COM12");
}
