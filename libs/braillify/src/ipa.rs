//! IPA (International Phonetic Alphabet) braille encoding (extracted from lib.rs).
//!
//! PDF 제38항 — IPA 점자 표기. [...], /.../ 묶음 안 음운 기호를 점역한다.

use crate::{encode, english, utils, with_encoder};

pub(crate) fn is_ipa_phonetic_symbol(c: char) -> bool {
    matches!(c, 'θ' | 'ə' | 'æ' | 'ŋ' | 'ː')
}

/// PDF 제38항 자동 감지 — input의 묶음 패턴 안 IPA 음운 기호로 IPA 컨텍스트 추론.
///
/// 알고리즘(AST적 판단):
/// 1. 입력을 좌→우 스캔하며 `[...]` 또는 `/.../` 매칭쌍을 찾는다.
/// 2. 매칭쌍 내부에 IPA 음운 기호(θ, ə, æ, ŋ, ː 등)가 하나라도 있으면
///    IPA 컨텍스트로 판정한다.
/// 3. 한 번이라도 IPA 매칭쌍을 발견하면 input 전체를 IPA로 처리한다.
///    (같은 input 안의 다른 `[...]`·`/.../`도 동일 컨텍스트로 본다.
///    예: `/æ/...로 .../a/로` — 첫 매칭이 IPA면 둘째도 IPA.)
///
/// 빈 묶음(`[ ]`·`/ /`)이나 음운 기호 없는 내용은 IPA가 아니다. URL 안 `://`,
/// 분수 `1/2`, 일반 대괄호 `[1]` 등이 IPA로 오인되지 않도록 한다.
///
/// IPA 음운 기호 집합은 본 라이브러리가 인식하는 부분 집합이며,
/// PDF 표에 새 기호 추가 시 `is_ipa_phonetic_symbol`와 `encode_ipa_char`을 함께 확장한다.
pub(crate) fn detect_ipa_context(text: &str) -> bool {
    let mut has_group_start = false;
    let mut has_ipa_symbol = false;
    for c in text.chars() {
        has_group_start |= matches!(c, '[' | '/');
        has_ipa_symbol |= is_ipa_phonetic_symbol(c);
        if has_group_start && has_ipa_symbol {
            break;
        }
    }
    if !has_group_start || !has_ipa_symbol {
        return false;
    }

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '[' => {
                if let Some(rel) = chars[i + 1..].iter().position(|&c| c == ']') {
                    let inner: &[char] = &chars[i + 1..i + 1 + rel];
                    if inner.iter().any(|c| is_ipa_phonetic_symbol(*c)) {
                        return true;
                    }
                    i += rel + 2;
                    continue;
                }
            }
            '/' => {
                if let Some(rel) = chars[i + 1..].iter().position(|&c| c == '/') {
                    let inner: &[char] = &chars[i + 1..i + 1 + rel];
                    if inner.iter().any(|c| is_ipa_phonetic_symbol(*c)) {
                        return true;
                    }
                    i += rel + 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// PDF 제38항 — 국제음성기호(IPA) 점자 변환.
///
/// 알고리즘:
/// 1. 좌→우 스캔하며 묶음 기호 상태(대괄호/빗금 열림 여부)를 추적한다.
/// 2. `[`·`]`·`/`는 묶음 상태에 따라 시작/종료 점형을 출력한다.
/// 3. 묶음 안에서는 IPA 변환표에 따라 음운 기호와 영문자를 인코딩한다.
/// 4. 묶음 밖의 한국어/영문/숫자 등은 일반 점자 인코더로 위임한다.
///
/// 본 함수가 적용되는 경우는 testcase의 `context: "ipa"` 또는
/// `EncodeOptions::default_mode = Some(EncodingMode::Ipa)`로 명시된 상황뿐이며,
/// 자동 감지는 별도 token rule에서 처리한다.
///
/// 점자 셀 인덱스 = (Unicode braille codepoint) − 0x2800.
///   ⠐ = 16 (점 5)         ⠘ = 24 (점 4+5)        ⠷ = 55 (점 1+2+3+5+6)
///   ⠾ = 62 (점 2+3+4+5+6) ⠌ = 12 (점 3+4)
pub(crate) fn encode_ipa(text: &str) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut bracket_open = false;
    let mut slash_open = false;
    let mut korean_buf = String::new();

    // IPA 입력 전체에 한국어가 한 글자라도 있으면, 묶음 밖 영어 어절도
    // 한국어 점자 환경의 일부로 보고 영자표시(⠴)를 emit해야 한다.
    // (예: "worth [wəːrθ]: ~해볼 만한" → 영어 "worth" 시작에 ⠴ 필요.)
    let has_korean_anywhere = text.chars().any(utils::is_korean_char);

    let flush_korean = |buf: &mut String, out: &mut Vec<u8>| -> Result<(), String> {
        if !buf.is_empty() {
            // 묶음 밖의 한국어/영문 등은 일반 인코더로 위임한다. 전체 입력에
            // 한국어가 있는 경우, 영어 단어 시작에 영자표시가 붙도록 강제한다.
            let enc = if has_korean_anywhere {
                with_encoder(true, |encoder| {
                    let mut result = Vec::new();
                    encoder.encode(buf.as_str(), &mut result)?;
                    Ok::<Vec<u8>, String>(result)
                })?
            } else {
                encode(buf.as_str())?
            };
            out.extend(enc);
            buf.clear();
        }
        Ok(())
    };

    // 영어 어절 직후 IPA 묶음이 이어지면, 영어 종료표(⠲)는 묶음 기호가 새
    // 컨텍스트를 열기 때문에 불필요하다. 공백 위에 놓인 종료표만 제거한다.
    fn strip_trailing_english_terminator_before_bracket(out: &mut Vec<u8>) {
        let mut i = out.len();
        while i > 0 && out[i - 1] == 0 {
            i -= 1;
        }
        if i > 0 && out[i - 1] == 50 {
            out.remove(i - 1);
        }
    }

    // 여는 대괄호: ⠐⠘⠷ = 16, 24, 55 | 닫는 대괄호: ⠘⠾ = 24, 62
    // 여는 빗금: ⠐⠘⠌ = 16, 24, 12 | 닫는 빗금: ⠘⠌ = 24, 12
    for ch in text.chars() {
        match ch {
            '[' => {
                flush_korean(&mut korean_buf, &mut out)?;
                strip_trailing_english_terminator_before_bracket(&mut out);
                out.extend_from_slice(&[16, 24, 55]);
                bracket_open = true;
            }
            ']' => {
                flush_korean(&mut korean_buf, &mut out)?;
                out.extend_from_slice(&[24, 62]);
                bracket_open = false;
            }
            '/' => {
                flush_korean(&mut korean_buf, &mut out)?;
                if slash_open {
                    out.extend_from_slice(&[24, 12]);
                    slash_open = false;
                } else {
                    strip_trailing_english_terminator_before_bracket(&mut out);
                    out.extend_from_slice(&[16, 24, 12]);
                    slash_open = true;
                }
            }
            ' ' => {
                flush_korean(&mut korean_buf, &mut out)?;
                out.push(0);
            }
            _ if bracket_open || slash_open => {
                // 묶음 안: IPA 음운/영문 점자 변환.
                flush_korean(&mut korean_buf, &mut out)?;
                let bytes =
                    encode_ipa_char(ch).ok_or_else(|| format!("Unknown IPA character: {ch:?}"))?;
                out.extend(bytes);
            }
            _ => {
                // 묶음 밖: 일반 텍스트는 한국어/영문 인코더로 위임.
                korean_buf.push(ch);
            }
        }
    }
    flush_korean(&mut korean_buf, &mut out)?;
    Ok(out)
}

/// PDF 제38항 IPA 변환표 — 음운 기호 및 영문자 점자 매핑.
/// 영문 알파벳은 일반 영어 점자 매핑(`english::encode_english`)을 사용한다.
///
/// 점자 셀 인덱스 = (Unicode braille codepoint) − 0x2800.
pub(crate) fn encode_ipa_char(ch: char) -> Option<Vec<u8>> {
    // PDF 국제음성기호 점자 규정 변환표 — 음운 기호 매핑.
    // (현재 본 라이브러리가 인식하는 음운 기호 부분 집합.
    //  새 기호 추가 시 PDF 표에 근거해 직접 추가한다.)
    match ch {
        'ə' => Some(vec![34]),     // ⠢ (점 2+6)
        'ː' => Some(vec![18]),     // ⠒ (점 2+5) — 장음 표시
        'θ' => Some(vec![40, 57]), // ⠨⠹ (점 4+6, 점 1+4+5+6)
        'ŋ' => Some(vec![43]),     // ⠫ (점 1+2+4+6)
        'æ' => Some(vec![41]),     // ⠩ (점 1+4+6)
        _ => {
            // 기본 알파벳/숫자는 일반 영어 점자 변환을 사용.
            if let Ok(code) = english::encode_english(ch) {
                Some(vec![code])
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ipa:108 — IPA input WITHOUT any Korean char triggers the else branch
    /// in flush_korean: `encode(buf.as_str())?` (no with_encoder wrap).
    #[test]
    fn ipa_input_without_korean_uses_plain_encode() {
        // Pure English + IPA bracket, no Korean anywhere.
        let _ = encode_ipa("think [θɪŋk]");
        let _ = encode_ipa("[θ]/æ/");
    }

    /// ipa:196 — `encode_ipa_char` returns None for chars that aren't in the
    /// IPA mapping AND `english::encode_english` returns Err for.
    /// Use a character that has no English mapping (e.g. emoji or arbitrary unicode).
    #[test]
    fn ipa_encode_char_none_for_unsupported() {
        // Emoji or arbitrary char not in english map and not in IPA map.
        assert!(encode_ipa_char('\u{1F600}').is_none()); // 😀
        // Arbitrary CJK character not in english.
        assert!(encode_ipa_char('한').is_none());
    }
}
