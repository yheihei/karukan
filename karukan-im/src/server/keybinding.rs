use tracing::warn;

use crate::config::settings::KeyBindingSettings;
use crate::core::keycode::{KeyEvent, KeyModifiers, Keysym};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SemanticAction {
    ToHiragana,
    ToKatakana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyBinding {
    keysym: Keysym,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    fn matches(self, event: &KeyEvent) -> bool {
        event.is_press
            && normalize_letter(event.keysym) == self.keysym
            && event.modifiers == self.modifiers
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct MacKeyBindings {
    to_hiragana: Option<KeyBinding>,
    to_katakana: Option<KeyBinding>,
}

impl MacKeyBindings {
    pub(super) fn from_settings(settings: &KeyBindingSettings) -> Self {
        let mut bindings = Self {
            to_hiragana: parse_setting("to_hiragana", &settings.to_hiragana),
            to_katakana: parse_setting("to_katakana", &settings.to_katakana),
        };

        if bindings.to_hiragana.is_some() && bindings.to_hiragana == bindings.to_katakana {
            warn!("conflicting macOS keybindings for to_hiragana and to_katakana; disabling both");
            bindings.to_hiragana = None;
            bindings.to_katakana = None;
        }
        bindings
    }

    pub(super) fn action_for(&self, event: &KeyEvent) -> Option<SemanticAction> {
        if self
            .to_hiragana
            .is_some_and(|binding| binding.matches(event))
        {
            Some(SemanticAction::ToHiragana)
        } else if self
            .to_katakana
            .is_some_and(|binding| binding.matches(event))
        {
            Some(SemanticAction::ToKatakana)
        } else {
            None
        }
    }
}

fn parse_setting(name: &str, value: &str) -> Option<KeyBinding> {
    match parse_binding(value) {
        Ok(binding) => binding,
        Err(message) => {
            warn!("invalid macOS keybinding {name}={value:?}: {message}; disabling it");
            None
        }
    }
}

fn parse_binding(value: &str) -> Result<Option<KeyBinding>, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    if value.is_empty() {
        return Err("binding is empty".to_string());
    }

    let mut modifiers = KeyModifiers::default();
    let mut keysym = None;
    for raw_part in value.split('+') {
        let part = raw_part.trim().to_ascii_lowercase();
        if part.is_empty() {
            return Err("binding contains an empty component".to_string());
        }
        match part.as_str() {
            "command" | "cmd" => set_modifier(&mut modifiers.super_key, &part)?,
            "control" | "ctrl" => set_modifier(&mut modifiers.control_key, &part)?,
            "option" | "alt" => set_modifier(&mut modifiers.alt_key, &part)?,
            "shift" => set_modifier(&mut modifiers.shift_key, &part)?,
            _ => {
                if keysym.is_some() {
                    return Err("binding must contain exactly one non-modifier key".to_string());
                }
                keysym = Some(parse_key(&part)?);
            }
        }
    }

    if !modifiers.super_key && !modifiers.control_key && !modifiers.alt_key {
        return Err("command, control, or option modifier is required".to_string());
    }
    let keysym = keysym.ok_or_else(|| "binding has no non-modifier key".to_string())?;
    Ok(Some(KeyBinding { keysym, modifiers }))
}

fn set_modifier(target: &mut bool, name: &str) -> Result<(), String> {
    if *target {
        return Err(format!("duplicate modifier {name}"));
    }
    *target = true;
    Ok(())
}

fn parse_key(value: &str) -> Result<Keysym, String> {
    let named = match value {
        "space" => Some(Keysym::SPACE),
        "tab" => Some(Keysym::TAB),
        "return" | "enter" => Some(Keysym::RETURN),
        "escape" | "esc" => Some(Keysym::ESCAPE),
        "backspace" => Some(Keysym::BACKSPACE),
        "delete" => Some(Keysym::DELETE),
        "home" => Some(Keysym::HOME),
        "end" => Some(Keysym::END),
        "left" => Some(Keysym::LEFT),
        "right" => Some(Keysym::RIGHT),
        "up" => Some(Keysym::UP),
        "down" => Some(Keysym::DOWN),
        "pageup" | "page_up" => Some(Keysym::PAGE_UP),
        "pagedown" | "page_down" => Some(Keysym::PAGE_DOWN),
        "plus" => Some(Keysym('+' as u32)),
        "f1" => Some(Keysym(0xffbe)),
        "f2" => Some(Keysym(0xffbf)),
        "f3" => Some(Keysym(0xffc0)),
        "f4" => Some(Keysym(0xffc1)),
        "f5" => Some(Keysym(0xffc2)),
        "f6" => Some(Keysym(0xffc3)),
        "f7" => Some(Keysym(0xffc4)),
        "f8" => Some(Keysym(0xffc5)),
        "f9" => Some(Keysym(0xffc6)),
        "f10" => Some(Keysym(0xffc7)),
        "f11" => Some(Keysym(0xffc8)),
        "f12" => Some(Keysym(0xffc9)),
        _ => None,
    };
    if let Some(keysym) = named {
        return Ok(keysym);
    }

    let mut chars = value.chars();
    let ch = chars
        .next()
        .ok_or_else(|| "binding has no non-modifier key".to_string())?;
    if chars.next().is_some() || !ch.is_ascii_graphic() {
        return Err(format!("unsupported key {value:?}"));
    }
    Ok(Keysym(ch.to_ascii_lowercase() as u32))
}

fn normalize_letter(keysym: Keysym) -> Keysym {
    let value = keysym.0;
    if (b'A' as u32..=b'Z' as u32).contains(&value) {
        Keysym(value + u32::from(b'a' - b'A'))
    } else {
        keysym
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(keysym: Keysym, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(keysym, modifiers, true)
    }

    #[test]
    fn parses_aliases_and_matches_modifiers_exactly() {
        let binding = parse_binding("Cmd+Ctrl+Shift+K").unwrap().unwrap();
        let modifiers = KeyModifiers {
            shift_key: true,
            control_key: true,
            alt_key: false,
            super_key: true,
        };
        assert!(binding.matches(&event(Keysym::KEY_K_UPPER, modifiers)));

        let with_alt = KeyModifiers {
            alt_key: true,
            ..modifiers
        };
        assert!(!binding.matches(&event(Keysym::KEY_K_UPPER, with_alt)));
    }

    #[test]
    fn supports_none_and_named_keys() {
        assert_eq!(parse_binding("none").unwrap(), None);
        assert_eq!(
            parse_binding("option+space").unwrap().unwrap().keysym,
            Keysym::SPACE
        );
    }

    #[test]
    fn rejects_invalid_bindings() {
        assert!(parse_binding("j").is_err());
        assert!(parse_binding("shift+j").is_err());
        assert!(parse_binding("command+j+k").is_err());
        assert!(parse_binding("command+command+j").is_err());
    }

    #[test]
    fn conflicting_bindings_are_both_disabled() {
        let bindings = MacKeyBindings::from_settings(&KeyBindingSettings {
            to_hiragana: "command+j".to_string(),
            to_katakana: "cmd+J".to_string(),
        });
        assert!(bindings.to_hiragana.is_none());
        assert!(bindings.to_katakana.is_none());
    }
}
