use std::time::Duration;

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, Modifier};

use super::key_lookup::KeyLookup;
use super::key_names::keysym_for_char;
use super::keymap::Keymap;
use super::keymap_edit::KeymapEdit;
use super::keysym_step::KeysymStep;
use super::plan::Plan;
use super::step::Step;

pub(crate) const CHUNK_CHARS: usize = 32;
const REMAP_SETTLE: Duration = Duration::from_millis(30);
pub(crate) const REMAP_RESTORE_HOLD: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnmappedRoute {
    Remap,
    Refuse,
}

pub(crate) const UNMAPPED_ROUTE: UnmappedRoute = UnmappedRoute::Remap;

#[derive(Clone, Copy)]
enum Typing {
    Mapped(KeyLookup),
    Unmapped(u32),
}

struct Slot {
    keycode: u8,
    original: Vec<u32>,
}

struct Remap {
    slots: Vec<Slot>,
    width: usize,
}

pub(crate) const MAX_REMAP_SLOTS: usize = 32;

pub(crate) fn plan_text(map: &Keymap, text: &str) -> Result<Vec<Plan>, AdapterError> {
    plan_text_with(map, text, UNMAPPED_ROUTE)
}

pub(crate) fn plan_text_with(
    map: &Keymap,
    text: &str,
    route: UnmappedRoute,
) -> Result<Vec<Plan>, AdapterError> {
    let characters: Vec<char> = text.chars().collect();
    let typings: Vec<Typing> = characters.iter().map(|c| classify(map, *c)).collect();
    let unmapped: Vec<char> = characters
        .iter()
        .zip(&typings)
        .filter(|(_, typing)| matches!(typing, Typing::Unmapped(_)))
        .map(|(character, _)| *character)
        .collect();
    let mut distinct: Vec<u32> = Vec::new();
    for typing in &typings {
        if let Typing::Unmapped(keysym) = typing
            && !distinct.contains(keysym)
        {
            distinct.push(*keysym);
        }
    }
    let remap = if unmapped.is_empty() {
        None
    } else {
        Some(remap_for(map, &unmapped, &distinct, route)?)
    };
    characters
        .chunks(CHUNK_CHARS)
        .zip(typings.chunks(CHUNK_CHARS))
        .map(|(_, chunk)| chunk_plan(map, chunk, remap.as_ref()))
        .collect()
}

fn classify(map: &Keymap, character: char) -> Typing {
    let keysym = keysym_for_char(character);
    match map.lookup(keysym) {
        Some(lookup) => Typing::Mapped(lookup),
        None => Typing::Unmapped(keysym),
    }
}

fn remap_for(
    map: &Keymap,
    unmapped: &[char],
    keysyms: &[u32],
    route: UnmappedRoute,
) -> Result<Remap, AdapterError> {
    if route == UnmappedRoute::Refuse || unmapped.iter().any(|c| c.is_control()) {
        return Err(refusal(unmapped));
    }
    let spare = map.spare_keycodes(keysyms.len().min(MAX_REMAP_SLOTS));
    if spare.is_empty() {
        return Err(AdapterError::new(
            ErrorCode::ActionNotSupported,
            "The keyboard layout has no spare key to type characters it lacks",
        )
        .with_details(serde_json::json!({ "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    let slots = spare
        .into_iter()
        .map(|keycode| {
            map.keysyms_of(keycode)
                .map(|original| Slot { keycode, original })
                .ok_or_else(|| refusal(unmapped))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Remap {
        slots,
        width: usize::from(map.keysyms_per_keycode),
    })
}

struct Assignments<'a> {
    remap: &'a Remap,
    holds: Vec<Option<u32>>,
    next: usize,
}

impl<'a> Assignments<'a> {
    fn new(remap: &'a Remap) -> Self {
        Self {
            remap,
            holds: vec![None; remap.slots.len()],
            next: 0,
        }
    }

    fn keycode_for(&mut self, steps: &mut Vec<Step>, keysym: u32) -> u8 {
        if let Some(index) = self.holds.iter().position(|held| *held == Some(keysym)) {
            return self.remap.slots[index].keycode;
        }
        let index = self.next;
        self.next = (self.next + 1) % self.remap.slots.len();
        if self.holds[index].is_some() {
            steps.push(Step::Pause(REMAP_RESTORE_HOLD));
        }
        let keycode = self.remap.slots[index].keycode;
        steps.push(Step::Keymap(KeymapEdit {
            keycode,
            keysyms: vec![keysym; self.remap.width],
        }));
        steps.push(Step::Pause(REMAP_SETTLE));
        self.holds[index] = Some(keysym);
        keycode
    }

    fn restore(&self, steps: &mut Vec<Step>) {
        steps.push(Step::Pause(REMAP_RESTORE_HOLD));
        for (index, held) in self.holds.iter().enumerate() {
            if held.is_some() {
                steps.push(Step::Keymap(KeymapEdit {
                    keycode: self.remap.slots[index].keycode,
                    keysyms: self.remap.slots[index].original.clone(),
                }));
            }
        }
    }
}

fn chunk_plan(map: &Keymap, chunk: &[Typing], remap: Option<&Remap>) -> Result<Plan, AdapterError> {
    let shift = map.modifier_code(&Modifier::Shift);
    let mut steps = Vec::new();
    let mut assignments = remap.map(Assignments::new);
    for typing in chunk {
        match *typing {
            Typing::Mapped(lookup) => mapped_steps(&mut steps, lookup, shift)?,
            Typing::Unmapped(keysym) => {
                let Some(assignments) = assignments.as_mut() else {
                    return Err(AdapterError::new(
                        ErrorCode::Internal,
                        "An unmapped character reached planning without a remap",
                    ));
                };
                let code = assignments.keycode_for(&mut steps, keysym);
                steps.push(Step::Key { code, down: true });
                steps.push(Step::Key { code, down: false });
            }
        }
    }
    if let Some(assignments) = assignments.filter(|a| a.holds.iter().any(Option::is_some)) {
        assignments.restore(&mut steps);
    }
    Ok(Plan {
        steps,
        release_origin: None,
    })
}

fn mapped_steps(
    steps: &mut Vec<Step>,
    lookup: KeyLookup,
    shift: Option<u8>,
) -> Result<(), AdapterError> {
    let wrapper = match (lookup.shift, shift) {
        (true, None) => {
            return Err(AdapterError::new(
                ErrorCode::ActionNotSupported,
                "The keyboard has no Shift key to type this text",
            )
            .with_details(serde_json::json!({ "physical_delivery_started": false }))
            .with_disposition(DeliverySemantics::not_delivered()));
        }
        (true, Some(code)) => Some(code),
        (false, _) => None,
    };
    if let Some(code) = wrapper {
        steps.push(Step::Key { code, down: true });
    }
    steps.push(Step::Key {
        code: lookup.code,
        down: true,
    });
    steps.push(Step::Key {
        code: lookup.code,
        down: false,
    });
    if let Some(code) = wrapper {
        steps.push(Step::Key { code, down: false });
    }
    Ok(())
}

fn refusal(unmapped: &[char]) -> AdapterError {
    let mut classes: Vec<&'static str> = unmapped.iter().map(|c| character_class(*c)).collect();
    classes.sort_unstable();
    classes.dedup();
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "Text contains characters the active keyboard layout cannot type",
    )
    .with_details(serde_json::json!({
        "physical_delivery_started": false,
        "unmapped_character_classes": classes,
        "unmapped_character_count": unmapped.len(),
    }))
    .with_suggestion(
        "Type only characters on the active keyboard layout, or set the value directly",
    )
    .with_disposition(DeliverySemantics::not_delivered())
}

fn character_class(character: char) -> &'static str {
    if character.is_control() {
        "control"
    } else if u32::from(character) < 0x250 {
        "latin_extended"
    } else if character.is_alphabetic() {
        "other_script_letter"
    } else {
        "symbol_or_emoji"
    }
}

pub(crate) fn plan_text_keysyms(text: &str) -> Vec<Vec<KeysymStep>> {
    let characters: Vec<char> = text.chars().collect();
    characters
        .chunks(CHUNK_CHARS)
        .map(|chunk| {
            chunk
                .iter()
                .flat_map(|character| {
                    let keysym = keysym_for_char(*character);
                    [
                        KeysymStep { keysym, down: true },
                        KeysymStep {
                            keysym,
                            down: false,
                        },
                    ]
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
#[path = "text_plan_tests.rs"]
mod tests;
