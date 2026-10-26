// Small source-level regression checks for the application design system.
// The normal Linux/Windows CI pipeline separately compiles the full GUI.

// Optional animations are allowed only behind the user-controlled global
// performance gate. Their default state is OFF. This is intentionally
// stricter than allowing arbitrary animations, but unlike the v0.7.1 test it
// permits explicit, opt-in Fluent transitions.
fn assert_animations_use_effect_gate(name: &str, source: &str) {
    let mut remainder = source;
    while let Some(start) = remainder.find("animate ") {
        let declaration = &remainder[start..];
        let open = declaration
            .find('{')
            .expect("Slint animation declaration must have a body");
        let close = declaration[open + 1..]
            .find('}')
            .expect("Slint animation body must close");
        let body = &declaration[open + 1..open + 1 + close];
        assert!(
            body.contains("enabled: Theme.effects-enabled"),
            "{name}: animation bypasses global effects gate: {body}"
        );
        remainder = &declaration[open + 1 + close + 1..];
    }
}
#[test]
fn fluent2_application_surfaces_use_theme_tokens_and_gated_animations() {
    let build = include_str!("../build.rs");
    assert!(build.contains(".with_style(\"fluent\".into())"));
    assert!(build.contains("manifest_dir.join(\"ui/fluent/style\")"));
    assert!(build.contains("EmbedResourcesKind::EmbedFiles"));
    let theme = include_str!("../ui/theme.slint");
    assert!(theme.contains("accent-fill: #356f54"));
    assert!(theme.contains("effects-enabled: false"));
    for (name, source) in [
        ("app", include_str!("../ui/app.slint")),
        ("components", include_str!("../ui/components.slint")),
        ("results", include_str!("../ui/results.slint")),
        ("draw-classes", include_str!("../ui/draw-classes.slint")),
    ] {
        assert!(!source.contains("background: #"), "{name} bypasses theme");
        assert_animations_use_effect_gate(name, source);
        for effect in [
            "drop-shadow:",
            "opacity:",
            "linear-gradient(",
            "radial-gradient(",
        ] {
            assert!(!source.contains(effect), "{name} contains effect: {effect}");
        }
    }
}

#[test]
fn local_fluent_controls_have_no_ungated_effects_or_external_imports() {
    use std::path::Path;
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/fluent");
    fn check(directory: &Path, style: &Path) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                check(&path, style);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "slint")
            {
                let source = std::fs::read_to_string(&path).unwrap();
                assert_animations_use_effect_gate(&path.display().to_string(), &source);
                if source.contains("SlintInternal.") {
                    assert_eq!(path, style.join("style/color-scheme.slint"));
                    assert_eq!(source.matches("SlintInternal.").count(), 1);
                    assert!(source.contains("color-scheme: SlintInternal.color-scheme;"));
                }

                for line in source.lines().map(str::trim) {
                    if line.starts_with("//") {
                        continue;
                    }
                    for effect in [
                        "drop-shadow-",
                        "opacity:",
                        "linear-gradient(",
                        "radial-gradient(",
                        ".with-alpha(",
                    ] {
                        assert!(!line.contains(effect), "{}: {line}", path.display());
                    }
                    // Every widget import must resolve locally, preventing a
                    // silent fallback to the toolkit's animated default style.
                    if let Some((_, import)) = line.split_once(" from \"") {
                        let name = import.split('"').next().unwrap();
                        let relative = path.parent().unwrap().join(name);
                        let standard = style.join("style").join(name);
                        assert!(
                            relative.is_file() || standard.is_file(),
                            "{}: unresolved local import {name}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
    check(&style, &style);
}

#[test]
fn effects_preference_is_opt_in_and_is_saved_outside_competition_data() {
    let theme = include_str!("../ui/theme.slint");
    let app = include_str!("../ui/app.slint");
    let prefs = include_str!("../src/preferences.rs");
    assert!(theme.contains("effects-enabled: false"));
    assert!(app.contains("Ota animaatiot ja tehosteet käyttöön"));
    assert!(app.contains("Theme.effects-enabled = root.effects-enabled"));
    assert!(prefs.contains("pub effects_enabled: bool"));
    assert!(prefs.contains("pub fn save_effects_enabled"));
}

#[test]
fn ordinary_text_and_primary_action_have_readable_contrast() {
    // Evaluate the actual shared palette rather than freezing every color.
    let theme = include_str!("../ui/theme.slint");
    for name in [
        "SpruceColors",
        "LightColors",
        "DarkColors",
        "HiillosColors",
        "ShanghaiColors",
    ] {
        let palette = theme
            .split(&format!("export global {name} {{"))
            .nth(1)
            .unwrap()
            .split('}')
            .next()
            .unwrap();
        let luminance = |name: &str| {
            let token = format!(" {name}: #");
            let hex = palette.split(&token).nth(1).expect("theme color token");
            let mut channels = [0.0; 3];
            for (index, channel) in channels.iter_mut().enumerate() {
                let value =
                    u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap() as f64 / 255.0;
                *channel = if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                };
            }
            channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
        };
        for (foreground, background) in [
            ("ink", "canvas"),
            ("ink", "surface"),
            ("muted", "surface"),
            ("primary-ink", "accent-fill"),
            ("primary-ink", "accent-hover"),
            ("accent-text", "accent-soft"),
        ] {
            let a = luminance(foreground);
            let b = luminance(background);
            let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            assert!(
                contrast >= 4.5,
                "{foreground} / {background}: text contrast is only {contrast:.2}:1"
            );
        }
    }
}

#[test]
fn route_details_and_table_mouse_selection_remain_distinct() {
    let app = include_str!("../ui/app.slint");
    let components = include_str!("../ui/components.slint");
    assert!(app.contains("fields: root.route-fields;"));
    assert!(app.contains("root.route-field-commit(key, value)"));
    assert!(components.contains("touch := TouchArea { clicked => { root.selected(row.key); } }"));
    // Scope the check to the actual table rather than other accessible controls.
    let table = components
        .split("export component DataTable inherits Rectangle {")
        .nth(1)
        .and_then(|section| section.split("component EditorEntry").next())
        .expect("DataTable definition");
    assert!(!table.contains("FocusScope"));
    assert!(!table.contains("forward-focus"));
}

#[test]
fn editor_widths_do_not_depend_on_window_width() {
    let app = include_str!("../ui/app.slint");
    // Referencing root.width from a child inside the root's layout creates a
    // layoutinfo-h <-> width feedback loop in Slint 1.16.
    // Beta: Workspace owns panel geometry behind explicit minimum constraints.
    for line in app.lines() {
        let declaration = line.trim_start();
        assert!(
            !(declaration.starts_with("width:") && declaration.contains("root.width *")),
            "window-dependent editor width creates a binding loop: {declaration}"
        );
    }
}

#[test]
fn fade_surface_is_opt_in_and_does_not_change_input_or_data() {
    let motion = include_str!("../ui/motion.slint");
    assert_animations_use_effect_gate("motion", motion);
    assert!(motion.contains("motion-enabled: Theme.effects-enabled"));
    assert!(motion.contains("opacity: !root.motion-enabled || root.revealed ? 1 : 0"));
    assert!(motion.contains("running: root.motion-enabled && !root.revealed"));
    assert!(motion.contains("duration: root.revealed ? Theme.duration-normal : 0ms"));
    for forbidden in [
        "TouchArea",
        "FocusScope",
        "field-commit",
        "navigate(",
        "drop-shadow",
        "linear-gradient",
    ] {
        assert!(
            !motion.contains(forbidden),
            "motion layer must not own input or competition state: {forbidden}"
        );
    }
}
