use super::*;

#[test]
fn empty_preedit_does_not_consume_script_conversion() {
    let mut engine = InputMethodEngine::new();

    let result = engine.transform_preedit(KanaScript::Katakana);

    assert!(!result.consumed);
    assert!(matches!(engine.state(), InputState::Empty));
}

#[test]
fn transforms_current_preedit_and_resumes_hiragana_input() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));

    let result = engine.transform_preedit(KanaScript::Katakana);

    assert!(result.consumed);
    assert_eq!(engine.preedit().unwrap().text(), "アイ");
    assert_eq!(engine.mode.current(), InputMode::Hiragana);

    engine.process_key(&press('u'));
    assert_eq!(engine.preedit().unwrap().text(), "アイう");
}

#[test]
fn to_hiragana_exits_persistent_katakana_mode() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_ctrl(Keysym::KEY_K));
    assert_eq!(engine.preedit().unwrap().text(), "アイ");

    engine.transform_preedit(KanaScript::Hiragana);

    assert_eq!(engine.preedit().unwrap().text(), "あい");
    assert_eq!(engine.mode.current(), InputMode::Hiragana);
}

#[test]
fn transforms_only_kana_and_preserves_cursor() {
    let mut engine = InputMethodEngine::new();
    engine.input_buf.insert("あA・漢ア");
    engine.input_buf.set_cursor(2);
    engine.set_composing_state();

    engine.transform_preedit(KanaScript::Katakana);

    assert_eq!(engine.preedit().unwrap().text(), "アA・漢ア");
    assert_eq!(engine.preedit().unwrap().caret(), 2);

    engine.transform_preedit(KanaScript::Hiragana);
    assert_eq!(engine.preedit().unwrap().text(), "あA・漢あ");
    assert_eq!(engine.preedit().unwrap().caret(), 2);
}

#[test]
fn consumes_non_kana_preedit_without_changing_it() {
    let mut engine = InputMethodEngine::new();
    engine.input_buf.insert("ABC123");
    engine.set_composing_state();

    let result = engine.transform_preedit(KanaScript::Katakana);

    assert!(result.consumed);
    assert_eq!(engine.preedit().unwrap().text(), "ABC123");
}

#[test]
fn flushes_pending_romaji_before_transforming() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('k'));

    let result = engine.transform_preedit(KanaScript::Katakana);

    assert!(result.consumed);
    assert_eq!(engine.preedit().unwrap().text(), "k");
    assert_eq!(engine.input_buf.pending(), "");
}

#[test]
fn conversion_state_uses_original_reading_and_hides_candidates() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.process_key(&press('i'));
    engine.process_key(&press_key(Keysym::SPACE));
    assert!(matches!(engine.state(), InputState::Conversion { .. }));

    let result = engine.transform_preedit(KanaScript::Katakana);

    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(engine.preedit().unwrap().text(), "アイ");
    assert!(engine.candidates().is_none());
    assert!(
        result
            .actions
            .iter()
            .any(|action| matches!(action, EngineAction::HideCandidates))
    );
}

#[test]
fn script_conversion_clears_live_conversion_display() {
    let mut engine = InputMethodEngine::new();
    engine.process_key(&press('a'));
    engine.chunks = vec![ComposingChunk {
        reading: "あ".to_string(),
        converted: "亜".to_string(),
    }];
    engine.live.shown = true;
    engine.set_composing_state();
    assert_eq!(engine.preedit().unwrap().text(), "亜");

    engine.transform_preedit(KanaScript::Katakana);

    assert!(!engine.live.shown);
    assert!(engine.chunks.is_empty());
    assert_eq!(engine.preedit().unwrap().text(), "ア");
}
