use as2015_rs::{application, native};
use std::path::Path;

#[test]
fn synthetic_toml_uses_the_existing_scoring_core() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/native-demo/kilpailu.toml");
    let archive = native::load(&p).unwrap();
    assert_eq!(archive.basic.identity[1], "Esimerkkikilpailu");
    assert_eq!(archive.routes[0].index, 1);
    let report = application::calculate(&archive).unwrap();
    let total = report.competitors[0]
        .calculation
        .as_ref()
        .unwrap()
        .totals
        .total_points;
    assert_eq!(total, 645);
}

#[test]
fn unknown_native_format_version_is_rejected() {
    let original = include_str!("../fixtures/native-demo/kilpailu.toml");
    let replacement = original.replacen("versio = 1", "versio = 99", 1);
    let project: native::Project = toml::from_str(&replacement).unwrap();
    assert!(project.into_archive().is_err());
}

#[test]
fn typo_in_field_is_not_silently_ignored() {
    let original = include_str!("../fixtures/native-demo/kilpailu.toml");
    let replacement = original.replacen("ajanottotapa = 1", "ajanottotapp = 1", 1);
    assert!(toml::from_str::<native::Project>(&replacement).is_err());
}
