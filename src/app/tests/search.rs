use super::*;

// ---- Search tests ----

#[test]
fn search_mode_keeps_s_as_query_text() {
    let mut app = make_app_with_names(&["sample.png"]);

    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Name);
    assert_eq!(app.search.as_ref().unwrap().query, "s");
}

#[test]
fn search_mode_keeps_r_as_query_text() {
    let mut app = make_app_with_names(&["sample.png"]);

    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);

    assert!(app.rename.is_none());
    assert_eq!(app.search.as_ref().unwrap().query, "r");
}

#[test]
fn test_search_triggers_on_slash() {
    let mut app = make_app(20);
    assert!(app.search.is_none());
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(app.search.is_some());
    assert_eq!(app.search.as_ref().unwrap().trigger_char, '/');
}

#[test]
fn test_search_triggers_on_backslash() {
    let mut app = make_app(20);
    app.handle_key(KeyCode::Char('\\'), KeyModifiers::NONE);
    assert!(app.search.is_some());
    assert_eq!(app.search.as_ref().unwrap().trigger_char, '\\');
}

#[test]
fn test_search_esc_exits_search() {
    let mut app = make_app(20);
    app.selected = 10;
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(app.search.is_some());
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.search.is_none());
    assert_eq!(app.selected, 10);
}

#[test]
fn test_search_char_jumps_and_pushes_to_query() {
    let mut app = make_app(20);
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);
    let search = app.search.as_ref().unwrap();
    assert_eq!(search.query, "0");
    assert!(!search.matches.is_empty());
}

#[test]
fn test_search_backspace_works() {
    let mut app = make_app(20);
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    let search = app.search.as_ref().unwrap();
    assert_eq!(search.query, "");
}

#[test]
fn test_search_tab_cycles_matches() {
    let mut app = make_app_with_names(&["a_a.png", "a_b.png", "a_c.png", "x.png"]);
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
    let first_match_idx = app.search.as_ref().unwrap().match_idx;
    app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    let search = app.search.as_ref().unwrap();
    let expected = (first_match_idx + 1) % search.matches.len();
    assert_eq!(search.match_idx, expected);
}
