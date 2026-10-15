//! Sisältövertailu ei rinnasta lukijan vaihtuvaa 250-aikaa kortin leimauksiin.
use as2026::{CoreError, Punch, emit};

fn frame(reader_seconds: u16) -> emit::EmitFrame {
    let mut decoded = [0u8; emit::FRAME_LENGTH];
    decoded[..2].fill(0xff);
    decoded[2..5].copy_from_slice(&538747u32.to_le_bytes()[..3]);
    let records = [
        (0, 0u16),
        (47, 145),
        (48, 207),
        (49, 238),
        (50, 267),
        (100, 396),
        (250, reader_seconds),
    ];
    for (index, (code, elapsed)) in records.iter().enumerate() {
        let offset = 10 + index * 3;
        decoded[offset] = *code;
        decoded[offset + 1..offset + 3].copy_from_slice(&elapsed.to_le_bytes());
    }
    emit::parse_frame(&decoded.map(|byte| byte ^ emit::XOR_MASK)).unwrap()
}

#[test]
fn reread_with_changed_reader_time_has_the_same_card_contents() {
    let first = frame(5122);
    let second = frame(5239);
    assert_ne!(first.raw, second.raw);
    assert_eq!(first.card_number, 538747);
    assert_eq!(
        first.punches_before_reader_marker().unwrap(),
        second.punches_before_reader_marker().unwrap()
    );
    assert_eq!(
        first.punches_before_reader_marker().unwrap()[5],
        Punch {
            code: 100,
            elapsed_seconds: 396,
        }
    );
}

#[test]
fn changed_punch_or_anchor_is_detected_but_trailing_padding_is_ignored() {
    let original = frame(5122);
    let mut changed = original.clone();
    changed.records[2].elapsed_seconds += 1;
    assert_ne!(
        original.punches_before_reader_marker().unwrap(),
        changed.punches_before_reader_marker().unwrap()
    );
    changed = original.clone();
    changed.records[0].elapsed_seconds += 1;
    assert_ne!(
        original.punches_before_reader_marker().unwrap(),
        changed.punches_before_reader_marker().unwrap()
    );
    changed = original.clone();
    changed.records[49] = Punch {
        code: 99,
        elapsed_seconds: 1,
    };
    assert_eq!(
        original.punches_before_reader_marker().unwrap(),
        changed.punches_before_reader_marker().unwrap()
    );
}

#[test]
fn invalid_contents_have_no_accepted_punch_slice() {
    let mut invalid = frame(5122);
    invalid.records[0].code = 250;
    assert!(matches!(
        invalid.punches_before_reader_marker(),
        Err(CoreError::MarkerAtStart)
    ));
    invalid.records.fill(Punch::default());
    assert!(matches!(
        invalid.punches_before_reader_marker(),
        Err(CoreError::MissingMarker)
    ));
}
