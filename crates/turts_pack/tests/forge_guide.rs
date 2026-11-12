use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use turts_mod_api::{
    Binding, GuidePage, InputMapDef, InterfaceDef, Manifest, Primitive, Source, TextValue, Value,
    Widget,
};

fn forge_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")
}

fn manifest() -> Manifest {
    let path = forge_dir().join("manifest.ron");
    ron::from_str(&fs::read_to_string(path).expect("read Forge manifest"))
        .expect("parse Forge manifest")
}

fn interface() -> InterfaceDef {
    let manifest = manifest();
    assert_eq!(
        manifest.exports.interface.len(),
        1,
        "Forge must export one interface file with its guide"
    );
    let path = forge_dir().join(&manifest.exports.interface[0]);
    ron::from_str(&fs::read_to_string(path).expect("read exported Forge interface"))
        .expect("parse exported Forge interface")
}

fn effective_key_labels() -> BTreeSet<String> {
    let manifest = manifest();
    assert!(
        !manifest.exports.input.is_empty(),
        "Forge must export the input map used by its guide"
    );
    let mut input = InputMapDef {
        actions: Vec::new(),
        engine_overrides: Vec::new(),
    };
    for file in &manifest.exports.input {
        let path = forge_dir().join(file);
        let next: InputMapDef =
            ron::from_str(&fs::read_to_string(path).expect("read exported Forge input map"))
                .expect("parse exported Forge input map");
        input.actions.extend(next.actions);
        input.engine_overrides.extend(next.engine_overrides);
    }

    turts_interaction::input_map::effective_actions(&input)
        .values()
        .flatten()
        .filter_map(|binding| match binding {
            Binding::Key { code, .. } => Some(key_label(code)),
            Binding::Pointer { .. } | Binding::Wheel(_) => None,
        })
        .collect()
}

fn key_label(code: &str) -> String {
    match code {
        code if code.starts_with("Key") && code.len() == 4 => code[3..].to_owned(),
        code if code.starts_with("Digit") && code.len() == 6 => code[5..].to_owned(),
        "Escape" => "Esc".into(),
        "Space" => "Space".into(),
        "ControlLeft" | "ControlRight" => "Ctrl".into(),
        "ShiftLeft" | "ShiftRight" => "Shift".into(),
        other => other.to_owned(),
    }
}

fn guide_text(page: &GuidePage) -> Vec<String> {
    let mut text = vec![page.title.clone(), page.body.clone()];
    for widget in &page.widgets {
        widget_text(widget, &mut text);
    }
    text
}

fn widget_text(widget: &Widget, output: &mut Vec<String>) {
    if let Some(value) = &widget.tooltip {
        value_text(value, output);
    }
    match &widget.widget {
        Primitive::Box { children, .. } => {
            for child in children {
                widget_text(child, output);
            }
        }
        Primitive::Text { text, .. } => text_value(text, output),
        Primitive::Button { label, .. } => {
            if let Some(label) = label {
                text_value(label, output);
            }
        }
        Primitive::List { item, .. } | Primitive::Grid { item, .. } => widget_text(item, output),
        Primitive::Tooltip { value } => value_text(value, output),
        Primitive::Image { .. }
        | Primitive::Bar { .. }
        | Primitive::Cursor { .. }
        | Primitive::MinimapView { .. } => {}
    }
}

fn text_value(value: &TextValue, output: &mut Vec<String>) {
    match value {
        TextValue::Literal(text) | TextValue::Format(text, _) => output.push(text.clone()),
        TextValue::Bind(value) => value_text(value, output),
        TextValue::Item(_) => {}
    }
}

fn value_text(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Literal(text) | Value::Format(text, _) => output.push(text.clone()),
        Value::Item(_) => {}
        Value::Balance(_)
        | Value::Balances
        | Value::SelectionPrimary(_)
        | Value::SelectionUnits
        | Value::CommandCard
        | Value::BuildMenu
        | Value::ProductionQueue
        | Value::ControlGroups
        | Value::Alerts
        | Value::LastNotice
        | Value::Guide
        | Value::GuideOpen
        | Value::Bindings => {}
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn key_instruction_uses_single_letter(text: &str, label: &str) -> bool {
    let tokens = words(text);
    let label = label.to_ascii_lowercase();
    tokens.windows(2).any(|pair| {
        matches!(pair[0].as_str(), "press" | "hold" | "use" | "tap") && pair[1] == label
    })
}

fn guide_has_key_mention(text: &str, labels: &BTreeSet<String>) -> bool {
    let tokens = words(text);
    let named_specials = ["ctrl", "shift", "f1", "f10", "esc", "space", "tab"];
    if tokens
        .iter()
        .any(|word| named_specials.contains(&word.as_str()))
    {
        return true;
    }
    if labels
        .iter()
        .any(|label| label.len() >= 2 && tokens.iter().any(|word| word.eq_ignore_ascii_case(label)))
    {
        return true;
    }
    labels
        .iter()
        .any(|label| label.chars().count() == 1 && key_instruction_uses_single_letter(text, label))
}

fn contains_coordinate_pair(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("coordinate") {
        return true;
    }
    let bytes = lower.as_bytes();
    (0..bytes.len()).any(|start| {
        if !bytes[start].is_ascii_digit() {
            return false;
        }
        let mut index = start;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) != Some(&b',') {
            return false;
        }
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        bytes.get(index).is_some_and(u8::is_ascii_digit)
    })
}

fn has_bindings_list(widget: &Widget) -> bool {
    match &widget.widget {
        Primitive::List { source, item, .. } => {
            *source == Source::Bindings || has_bindings_list(item)
        }
        Primitive::Grid { item, .. } => has_bindings_list(item),
        Primitive::Box { children, .. } => children.iter().any(has_bindings_list),
        _ => false,
    }
}

#[test]
fn guide_pages_1_to_4_name_no_key() {
    let interface = interface();
    assert_eq!(interface.guide.len(), 5, "Forge guide must have five pages");
    let labels = effective_key_labels();
    let banned_studio_words = ["studio", "developer", "modder", "debug"];

    for (index, page) in interface.guide.iter().take(4).enumerate() {
        for text in guide_text(page) {
            let lower = text.to_ascii_lowercase();
            assert!(
                !guide_has_key_mention(&text, &labels),
                "guide page {} names a key in its teaching text: {text:?}",
                index + 1
            );
            assert!(
                !lower
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .any(|word| word == "ticks" || word == "applied"),
                "guide page {} uses an engine timing term: {text:?}",
                index + 1
            );
            assert!(
                !banned_studio_words.iter().any(|word| lower.contains(word)),
                "guide page {} uses a studio word: {text:?}",
                index + 1
            );
            assert!(
                !contains_coordinate_pair(&text),
                "guide page {} contains a coordinate: {text:?}",
                index + 1
            );
        }
    }
}

fn goal_box_text(interface: &InterfaceDef) -> Vec<String> {
    let goal = interface
        .root
        .iter()
        .find(|widget| widget.id == "objective")
        .expect("Forge HUD must have the goal box widget `objective`");
    let mut text = Vec::new();
    widget_text(goal, &mut text);
    text
}

#[test]
fn goal_box_names_no_key_and_names_each_side() {
    let interface = interface();
    let labels = effective_key_labels();
    let text = goal_box_text(&interface);
    assert!(
        text.iter().any(|line| line.starts_with("You are blue.")),
        "goal box must tell the blue side its colour: {text:?}"
    );
    assert!(
        text.iter().any(|line| line.starts_with("You are orange.")),
        "goal box must tell the orange side its colour: {text:?}"
    );
    for line in &text {
        let lower = line.to_ascii_lowercase();
        assert!(
            !guide_has_key_mention(line, &labels),
            "goal box names a key: {line:?}"
        );
        assert!(
            !contains_coordinate_pair(line),
            "goal box contains a coordinate: {line:?}"
        );
        assert!(
            !["studio", "developer", "modder", "debug", "ticks", "applied"]
                .iter()
                .any(|word| lower.contains(word)),
            "goal box uses an engine or studio word: {line:?}"
        );
    }
}

#[test]
fn guide_has_no_last_page_reference() {
    let interface = interface();
    for page in &interface.guide {
        for text in guide_text(page) {
            assert!(
                !text.to_ascii_lowercase().contains("last page"),
                "the guide is one block; page {:?} says \"last page\": {text:?}",
                page.id
            );
        }
    }
}

#[test]
fn guide_page_5_lists_bindings() {
    let interface = interface();
    assert_eq!(interface.guide.len(), 5, "Forge guide must have five pages");
    assert!(
        interface.guide[4].widgets.iter().any(has_bindings_list),
        "Forge guide page 5 must include a List sourced from Bindings"
    );
}

#[test]
fn guide_key_scan_keeps_plain_articles_and_finds_key_instructions() {
    let labels = BTreeSet::from(["A".to_owned(), "R".to_owned(), "F10".to_owned()]);
    assert!(!guide_has_key_mention("A worker feeds the press.", &labels));
    assert!(guide_has_key_mention("Press R to research.", &labels));
    assert!(guide_has_key_mention("Open the menu with F10.", &labels));
}
