pub mod difficulty;
pub mod notes;
pub mod rng;
pub mod rounds;
pub mod synth;
pub mod trainer;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use rounds::TestKind;
use trainer::Trainer;

/// JS-facing wrapper around [`Trainer`]. Holds no logic of its own.
#[wasm_bindgen]
pub struct Engine {
    trainer: Trainer,
}

fn parse_kind(kind: &str) -> Result<TestKind, JsError> {
    TestKind::parse(kind).ok_or_else(|| JsError::new(&format!("unknown test kind: {kind}")))
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32, seed: u32, saved_state_json: Option<String>) -> Engine {
        Engine {
            trainer: Trainer::new(sample_rate, seed as u64, saved_state_json.as_deref()),
        }
    }

    #[wasm_bindgen(js_name = newRound)]
    pub fn new_round(&mut self, kind: &str) -> Result<JsValue, JsError> {
        to_js(&self.trainer.new_round(parse_kind(kind)?))
    }

    #[wasm_bindgen(js_name = roundAudio)]
    pub fn round_audio(&self) -> Result<Vec<f32>, JsError> {
        Ok(self.trainer.round_audio()?)
    }

    pub fn answer(&mut self, answer: u32) -> Result<JsValue, JsError> {
        to_js(&self.trainer.answer(answer)?)
    }

    #[wasm_bindgen(js_name = noteAudio)]
    pub fn note_audio(&self, midi: u8) -> Vec<f32> {
        self.trainer.note_audio(midi)
    }

    #[wasm_bindgen(js_name = scaleNotes)]
    pub fn scale_notes(&self, from: u8, to: u8) -> Result<JsValue, JsError> {
        to_js(&self.trainer.scale_notes(from, to))
    }

    #[wasm_bindgen(js_name = scaleAudio)]
    pub fn scale_audio(&self, from: u8, to: u8) -> Vec<f32> {
        self.trainer
            .render_events(&self.trainer.scale_notes(from, to))
    }

    pub fn progress(&self, kind: &str) -> Result<JsValue, JsError> {
        to_js(&self.trainer.progress(parse_kind(kind)?))
    }

    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        self.trainer.state_json()
    }
}
