//! Static G09 schema predictions; no original AS2015A binary is executed.
use as2026::legacy::{Encoding, parse_basic, parse_competitors, parse_document};

fn parse(source: &str) -> as2026::legacy::Document {
    parse_document(source.as_bytes(), Encoding::Utf8).unwrap()
}

fn k_fields() -> Vec<&'static str> {
    vec![
        "12", "H", "12345", "Ari", "Katja", "Seura A", "Auto 1",
        "12", "0", "0", "0", "10", "0", "12", "10", "0",
        "45", "600", "2", "1200", "3", "75", "100", "2020",
        "1", "TRK", "", "0",
    ]
}

#[test]
fn k_header_maps_into_current_l_fields_and_retains_following_p_rows() {
    let p = [
        "1", "1", "L", "Lahto", "0", "0", "0", "0", "0",
        "12", "0", "0", "0", "", "0", "0", "0", "",
    ];
    let content = format!("K;{};\nP;{};\n", k_fields().join(";"), p.join(";"));
    let result = parse_competitors(&parse(&content)).unwrap();
    assert_eq!(result.len(), 1);
    let person = &result[0];
    assert_eq!(person.header_fields.len(), 31);
    assert_eq!(person.header_fields[5], "");
    assert_eq!(person.header_fields[6], "");
    assert_eq!(person.header_fields[7], "Seura A        ");
    assert_eq!(person.header_fields[8], "");
    assert_eq!(person.header_fields[9], "Auto 1");
    assert_eq!(person.header_fields[10], "12");
    assert_eq!(person.header_fields[19], "45");
    assert_eq!(person.header_fields[22], "1200");
    assert_eq!(person.total_penalties, 2020);
    assert_eq!(person.declared_row_count, 1);
    assert_eq!(person.status, "TRK");
    assert_eq!(person.results.len(), 1);
}

#[test]
fn mixed_legacy_k_and_l_headers_are_rejected_as_unsafe() {
    let normal = vec!["0"; 31];
    let content = format!("K;{};\nL;{};\n", k_fields().join(";"), normal.join(";"));
    let err = parse_competitors(&parse(&content)).unwrap_err();
    assert!(err.to_string().contains("sekoittaa"));
}

#[test]
fn pre_pe_legacy_basic_and_nine_column_classes_map_to_current_schema() {
    let source = "Syysajo;Jarjestaja;1;17.10.2026;0;2;\nH;1;12;15.00.00;2;0;60;1;4;\n";
    let settings = parse_basic(&parse(source)).unwrap();
    assert_eq!(settings.identity[0], "");
    assert_eq!(settings.identity[1], "Syysajo");
    assert_eq!(settings.identity[3], "Jarjestaja");
    assert_eq!(settings.identity[7], "2");
    assert_eq!(settings.classes.len(), 1);
    assert_eq!(settings.classes[0].fields, [
        "H", "H", "1", "12", "15.00.00", "2", "0", "60", "1", "4"
    ]);
    assert!(settings.combined_classes.is_empty());
}

#[test]
fn old_basic_rejects_new_tu_and_combined_class_records() {
    let source = "Syysajo;Jarjestaja;1;17.10.2026;0;2;\nTU;tulokset;asemat;\n";
    assert!(parse_basic(&parse(source)).is_err());
}
