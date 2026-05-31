use phf::phf_map;

use crate::unicode::decode_unicode;
static ENGLISH_SHORTCUT_MAP: phf::Map<&'static str, u8> = phf_map! {
    // 10.3
    "and" => decode_unicode('⠯'),
    "for" => decode_unicode('⠿'),
    "of" => decode_unicode('⠷'),
    "the" => decode_unicode('⠮'),
    "with" => decode_unicode('⠾'),
    // 10.4
    "ch" => decode_unicode('⠡'),
    "gh" => decode_unicode('⠣'),
    "sh" => decode_unicode('⠩'),
    "th" => decode_unicode('⠹'),
    "wh" => decode_unicode('⠱'),
    "ed" => decode_unicode('⠫'),
    "er" => decode_unicode('⠻'),
    "ou" => decode_unicode('⠳'),
    "ow" => decode_unicode('⠪'),
    "st" => decode_unicode('⠌'),
    "ing" => decode_unicode('⠬'),
    "ar" => decode_unicode('⠜'),


    // 10.6.1 - 하위 묶음 약자 - en, in
    "en" => decode_unicode('⠢'),
    "in" => decode_unicode('⠔'),
};

/// 10.3 - 온칸 약자
/// 10.4 - 온칸 묶음 약자
pub fn rule_en_10_4(current: &str) -> Option<(u8, usize)> {
    for key in ENGLISH_SHORTCUT_MAP.keys() {
        if current.starts_with(key) {
            return Some((*ENGLISH_SHORTCUT_MAP.get(key).unwrap(), key.len() - 1));
        }
    }
    None
}
// 한국 점자 PDF 제39항 영-한 점역에서 사용되는 1급 점자 하위 묶음 약자.
// UEB 표준의 'dis' 약자(⠲)는 testcase 점역 패턴상 미채택 (예: "dishes"는
// 'di-sh-es' 분리, 'dis-' prefix가 아닌 음절 단위로 점역). 한국 PDF의
// 영-한 점역이 'sh'(10.4 digraph) 같은 더 짧은 약자를 우선시한다고 본다.
static ENGLISH_SHORTCUT_MAP_10_6: phf::Map<&'static str, u8> = phf_map! {
    "ea" => decode_unicode('⠂'),
    "be" => decode_unicode('⠆'),
    "bb" => decode_unicode('⠆'),
    "con" => decode_unicode('⠒'),
    "cc" => decode_unicode('⠒'),
    "en" => decode_unicode('⠢'),
    "ff" => decode_unicode('⠖'),
    "gg" => decode_unicode('⠶'),
    "in" => decode_unicode('⠔'),
};

static ENGLISH_WHOLE_WORD_MAP_10_5: phf::Map<&'static str, &'static [u8]> = phf_map! {
    "every" => &[decode_unicode('⠐'), decode_unicode('⠑'), decode_unicode('⠽')],
    "knowledge" => &[
        decode_unicode('⠐'),
        decode_unicode('⠅'),
        decode_unicode('⠇'),
        decode_unicode('⠫'),
        decode_unicode('⠛'),
        decode_unicode('⠑'),
    ],
    "rather" => &[decode_unicode('⠗'), decode_unicode('⠁'), decode_unicode('⠮'), decode_unicode('⠗')],
    "enough" => &[decode_unicode('⠢'), decode_unicode('⠳'), decode_unicode('⠣')],
    "were" => &[decode_unicode('⠺'), decode_unicode('⠻'), decode_unicode('⠑')],
    "part" => &[decode_unicode('⠐'), decode_unicode('⠏')],
};

pub fn rule_en_10_5_whole_word(word: &str) -> Option<&'static [u8]> {
    ENGLISH_WHOLE_WORD_MAP_10_5.get(word).copied()
}
/// 10.6.1 - 하위 묶음 약자 - 시작할 때 일치 해야만 함
pub fn rule_en_10_6(current: &str) -> Option<(u8, usize)> {
    for key in ENGLISH_SHORTCUT_MAP_10_6.keys() {
        if current.starts_with(key) {
            return Some((*ENGLISH_SHORTCUT_MAP_10_6.get(key).unwrap(), key.len() - 1));
        }
    }
    None
}

/// 영-한 wrap context에서 사용되는 multi-cell 영어 약자.
/// 'ong'은 한국 점자 PDF 제39항이 점역하는 wordsign (⠰⠛).
static ENGLISH_MULTI_CELL_SHORTCUT: phf::Map<&'static str, &'static [u8]> = phf_map! {
    "ong" => &[decode_unicode('⠰'), decode_unicode('⠛')],
};

pub fn rule_en_multi_cell(current: &str) -> Option<(&'static [u8], usize)> {
    for (key, value) in ENGLISH_MULTI_CELL_SHORTCUT.entries() {
        if current.starts_with(*key) {
            return Some((*value, key.len() - 1));
        }
    }
    None
}
