// Small source-level regression checks for the effect-free design beta.
// The normal Linux/Windows CI pipeline separately compiles the full GUI.
#[test]
fn fluent2_beta_is_effect_free_and_uses_centralized_colors() {
    let build = include_str!("../build.rs");
    assert!(build.contains(".with_style(\"fluent\".into())"));
    let theme = include_str!("../ui/theme.slint");
    assert!(theme.contains("accent-fill: #356f54"));
    for (name, source) in [
        ("app", include_str!("../ui/app.slint")),
        ("components", include_str!("../ui/components.slint")),
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
fn route_details_and_table_mouse_selection_remain_distinct() {
    let app = include_str!("../ui/app.slint");
    let components = include_str!("../ui/components.slint");
    assert!(app.contains("if root.route-editor-visible: Editor"));
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
    assert!(app.contains("preferred-width: 344px;"));
    assert!(app.contains("preferred-width: 400px;"));
    for line in app.lines() {
        let declaration = line.trim_start();
        assert!(
            !(declaration.starts_with("width:") && declaration.contains("root.width *")),
            "window-dependent editor width creates a binding loop: {declaration}"
        );
    }
}
