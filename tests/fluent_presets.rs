// Contract checks for the Fluent 2 beta component preset library.
// Functional compilation, snapshots and mouse/keyboard smoke tests run in CI.
// These tests document invariant source relationships without approximating UI
// behavior as visual snapshots.
#[test]
fn common_presets_reuse_accessible_fluent_controls() {
    let presets = include_str!("../ui/presets.slint");
    for inherited in [
        "UiButton inherits Button",
        "UiTextField inherits LineEdit",
        "UiComboBox inherits ComboBox",
        "UiCheckBox inherits CheckBox",
        "UiScrollView inherits ScrollView",
    ] {
        assert!(presets.contains(inherited), "missing reuse: {inherited}");
    }
    for size in ["compact", "default", "large"] {
        assert!(presets.contains(size), "missing size {size}");
    }
    for variant in ["standard", "primary", "subtle", "danger"] {
        assert!(
            presets.contains(variant),
            "missing semantic variant {variant}"
        );
    }
    let button = include_str!("../ui/fluent/style/button.slint");
    assert!(button.contains("accessible-role: button"));
    assert!(button.contains("accessible-action-default"));
    assert!(button.contains("visual-variant"));
    assert!(button.contains("control-height"));
    assert!(button.contains("enabled: Theme.effects-enabled"));
}

#[test]
fn slider_has_real_accessibility_and_keyboard_contract() {
    let presets = include_str!("../ui/presets.slint");
    let slider = presets
        .split("export component UiSlider inherits Rectangle")
        .nth(1)
        .expect("UiSlider must have a concrete implementation");
    for expected in [
        "accessible-role: slider",
        "accessible-value-minimum",
        "accessible-value-maximum",
        "accessible-value-step",
        "accessible-action-increment",
        "accessible-action-decrement",
        "forward-focus: keyboard",
        "Key.LeftArrow",
        "Key.RightArrow",
        "Key.Home",
        "Key.End",
        "clicked =>",
        "moved =>",
        "callback changed(float)",
        "callback released(float)",
    ] {
        assert!(slider.contains(expected), "slider missing {expected}");
    }
    assert!(
        !slider.contains("animation-tick"),
        "slider must not schedule decorative animation"
    );
}

#[test]
fn gallery_is_developer_only_and_does_not_open_last_competition() {
    let app = include_str!("../ui/app.slint");
    let gallery = include_str!("../ui/fluent-gallery.slint");
    let rust = include_str!("../src/gui/mod.rs");
    assert!(app.contains("if root.gallery-open: FluentGallery"));
    assert!(rust.contains("Some(\"--fluent-gallery\")"));
    assert!(rust.contains("Some(\"--gallery-smoke-test\")"));
    assert!(
        rust.contains("let path = if fluent_gallery {")
            && rust.contains("None // Gallery never opens real competition data"),
        "gallery must not open the remembered competition"
    );
    assert!(gallery.contains("UiButton"));
    assert!(gallery.contains("UiSlider"));
    assert!(gallery.contains("UiScrollView"));
    assert!(gallery.contains("root.toggle-effects"));
    assert!(!gallery.contains("root.open-workspace"));
}

#[test]
fn every_gallery_animation_is_controlled_by_user_opt_in() {
    let theme = include_str!("../ui/theme.slint");
    let prefs = include_str!("../src/preferences.rs");
    let gallery = include_str!("../ui/fluent-gallery.slint");
    assert!(theme.contains("effects-enabled: false"));
    assert!(prefs.contains("pub effects_enabled: bool"));
    assert!(prefs.contains("pub fn save_effects_enabled"));
    assert!(gallery.contains("Ota animaatiot ja tehosteet käyttöön"));
    assert!(
        !gallery.contains("animate "),
        "gallery itself should not add optional effects"
    );
}
