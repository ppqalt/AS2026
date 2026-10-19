// Small source-level regression checks for the application design system.
// The normal Linux/Windows CI pipeline separately compiles the full GUI.
#[test]
fn fluent2_application_surfaces_are_effect_free_and_use_theme_tokens() {
    let build = include_str!("../build.rs");
    assert!(build.contains(".with_style(\"fluent\".into())"));
    assert!(build.contains("manifest_dir.join(\"ui/fluent/style\")"));
    assert!(build.contains("EmbedResourcesKind::EmbedFiles"));
    let theme = include_str!("../ui/theme.slint");
    assert!(theme.contains("accent-fill: #356f54"));
    for (name, source) in [
        ("app", include_str!("../ui/app.slint")),
        ("components", include_str!("../ui/components.slint")),
        ("results", include_str!("../ui/results.slint")),
        ("draw-classes", include_str!("../ui/draw-classes.slint")),
    ] {
        assert!(!source.contains("background: #"), "{name} bypasses theme");
        for effect in [
            "animate ",
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
fn local_fluent_controls_have_no_decorative_effects_or_external_imports() {
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
                for line in source.lines().map(str::trim) {
                    if line.starts_with("//") {
                        continue;
                    }
                    for effect in [
                        "animate ",
                        "drop-shadow-",
                        "opacity:",
                        "linear-gradient(",
                        "radial-gradient(",
                        ".with-alpha(",
                        "SlintInternal.",
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
fn ordinary_text_and_primary_action_have_readable_contrast() {
    // Evaluate the actual shared palette rather than freezing every color.
    let theme = include_str!("../ui/theme.slint");
    let luminance = |name: &str| {
        let token = format!(" {name}: #");
        let hex = theme.split(&token).nth(1).expect("theme color token");
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
        ("ink", "accent-fill"),
        ("ink", "accent-hover"),
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
