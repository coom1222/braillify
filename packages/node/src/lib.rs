mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "encode")]
pub fn encode(text: &str) -> Result<Vec<u8>, String> {
    braillify::encode(text)
}

#[wasm_bindgen(js_name = "translateToUnicode")]
pub fn translate_to_unicode(text: &str) -> Result<String, String> {
    braillify::encode_to_unicode(text)
}

#[wasm_bindgen(js_name = "translateToBrailleFont")]
pub fn translate_to_braille_font(text: &str) -> Result<String, String> {
    braillify::encode_to_braille_font(text)
}

#[wasm_bindgen(js_name = "decodeFromUnicode")]
pub fn decode_from_unicode(braille: &str) -> Result<String, String> {
    braillify::decode(braille)
}
