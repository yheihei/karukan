//! Mode switching (katakana, alphabet, live conversion)

use tracing::debug;

use super::*;

impl InputMethodEngine {
    /// Convert only the current preedit to the requested kana script.
    pub(crate) fn transform_preedit(&mut self, script: KanaScript) -> EngineResult {
        let start = std::time::Instant::now();
        self.metrics.conversion_ms = 0;

        if matches!(self.state, InputState::Empty) {
            self.metrics.process_key_ms = start.elapsed().as_millis() as u64;
            return EngineResult::not_consumed();
        }

        self.flush_romaji_to_composed();
        self.input_buf.text = match script {
            KanaScript::Hiragana => karukan_engine::katakana_to_hiragana(&self.input_buf.text),
            KanaScript::Katakana => karukan_engine::hiragana_to_katakana(&self.input_buf.text),
        };

        // This is a one-shot conversion. New input in the same preedit resumes
        // normal hiragana input instead of inheriting the converted script.
        self.mode.set(InputMode::Hiragana);
        self.live.text.clear();
        self.chunks.clear();

        let preedit = self.set_composing_state();
        let result = EngineResult::consumed()
            .with_action(EngineAction::UpdatePreedit(preedit))
            .with_action(EngineAction::HideCandidates)
            .with_action(EngineAction::UpdateAuxText(self.format_aux_composing()));
        self.metrics.process_key_ms = start.elapsed().as_millis() as u64;
        result
    }

    /// Enter katakana mode (Ctrl+k)
    /// One-way switch to Katakana; a mode toggle key (Right Super, JIS 変換,
    /// macOS かな/right-⌘ tap) returns to Hiragana.
    pub(super) fn enter_katakana_mode(&mut self) -> EngineResult {
        // Already in katakana mode: nothing to do
        if self.mode.current() == InputMode::Katakana {
            return EngineResult::consumed();
        }

        self.mode.set(InputMode::Katakana);
        // Clear live conversion text so katakana mode takes priority on commit
        self.live.text.clear();

        let romaji_buffer = self.converters.romaji.buffer().to_string();

        if self.input_buf.text.is_empty() && romaji_buffer.is_empty() {
            return EngineResult::consumed();
        }

        let preedit = self.set_composing_state();

        // Update aux text to show mode
        let aux = format!("{} Karukan ({})", self.mode_indicator(), self.model_name());

        EngineResult::consumed()
            .with_action(EngineAction::UpdatePreedit(preedit))
            .with_action(EngineAction::UpdateAuxText(aux))
    }

    /// Toggle live conversion mode via Ctrl+Shift+L.
    ///
    /// When toggled ON during Composing, immediately convert the current
    /// input buffer so the user doesn't have to type another key to see the
    /// live result. When toggled OFF, drop any stale converted text so the
    /// preedit reverts to hiragana right away.
    pub(super) fn toggle_live_conversion(&mut self) -> EngineResult {
        self.live.enabled = !self.live.enabled;
        let mode = if self.live.enabled { "ON" } else { "OFF" };
        debug!("Live conversion toggled: {}", mode);
        let aux = EngineAction::UpdateAuxText(format!("ライブ変換: {}", mode));

        if matches!(self.state, InputState::Composing { .. })
            && self.mode.current() != InputMode::Katakana
        {
            if self.live.enabled {
                let mut result = self.refresh_input_state();
                result.actions.push(aux);
                return result;
            }
            if !self.live.text.is_empty() {
                self.live.text.clear();
                let preedit = self.set_composing_state();
                return EngineResult::consumed()
                    .with_action(EngineAction::UpdatePreedit(preedit))
                    .with_action(aux);
            }
        }

        EngineResult::consumed().with_action(aux)
    }
}
