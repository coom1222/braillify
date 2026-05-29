use phf::phf_map;

use crate::unicode::decode_unicode;

static SHORTCUT_MAP: phf::Map<char, &'static [u8]> = phf_map! {
    '"' => &[decode_unicode('⠦')],
    // '"' => &[decode_unicode('⠴')],
    '\'' => &[decode_unicode('⠠'), decode_unicode('⠦')],
    // '\'' => &[decode_unicode('⠴'), decode_unicode('⠄')],
    '~' => &[decode_unicode('⠈'), decode_unicode('⠔')],
    // PDF 제73항 붙임 1 — 빈칸 채우기 placeholder (U+F000, Private Use)
    // `⠸⠦⠦⠄` 형태로 점역한다 (4셀: 빈칸 표지 + 묶음 마커).
    '\u{F000}' => &[decode_unicode('⠸'), decode_unicode('⠦'), decode_unicode('⠦'), decode_unicode('⠄')],
    '…' => &[decode_unicode('⠠'), decode_unicode('⠠'), decode_unicode('⠠')],
    '⋯' => &[decode_unicode('⠠'), decode_unicode('⠠'), decode_unicode('⠠')],
    '!' => &[decode_unicode('⠖')],
    '.' => &[decode_unicode('⠲')],
    ',' => &[decode_unicode('⠐')],
    '?' => &[decode_unicode('⠦')],
    // PDF 제56항 — 드러냄표/굵은글자/점역자글자체 sentinels (expand_emphasis_marks가 삽입).
    '\u{E000}' => &[decode_unicode('⠠'), decode_unicode('⠤')], // 드러냄표 시작 (= 밑줄)
    '\u{E001}' => &[decode_unicode('⠤'), decode_unicode('⠄')], // 드러냄표 종료
    '\u{E002}' => &[decode_unicode('⠰'), decode_unicode('⠤')], // 굵은 글자 시작
    '\u{E003}' => &[decode_unicode('⠤'), decode_unicode('⠆')], // 굵은 글자 종료
    '\u{E004}' => &[decode_unicode('⠐'), decode_unicode('⠤')], // 점역자1 글자체 시작
    '\u{E005}' => &[decode_unicode('⠤'), decode_unicode('⠂')], // 점역자1 글자체 종료
    '\u{E006}' => &[decode_unicode('⠈'), decode_unicode('⠤')], // 점역자2 글자체 시작
    '\u{E007}' => &[decode_unicode('⠤'), decode_unicode('⠁')], // 점역자2 글자체 종료
    '“' => &[decode_unicode('⠦')],
    '”' => &[decode_unicode('⠴')],
    ':' => &[decode_unicode('⠐'), decode_unicode('⠂')],
    ';' => &[decode_unicode('⠰'), decode_unicode('⠆')],
    '_' => &[decode_unicode('⠤')],
    '*' => &[decode_unicode('⠐'), decode_unicode('⠔')],
    '(' => &[decode_unicode('⠦'), decode_unicode('⠄')],
    ')' => &[decode_unicode('⠠'), decode_unicode('⠴')],
    '{' => &[decode_unicode('⠦'), decode_unicode('⠂')],
    '}' => &[decode_unicode('⠐'), decode_unicode('⠴')],
    '[' => &[decode_unicode('⠦'), decode_unicode('⠆')],
    ']' => &[decode_unicode('⠰'), decode_unicode('⠴')],
    '〔' => &[decode_unicode('⠦'), decode_unicode('⠆')],
    '〕' => &[decode_unicode('⠰'), decode_unicode('⠴')],
    '·' => &[decode_unicode('⠐'), decode_unicode('⠆')],
    '：' => &[decode_unicode('⠐'), decode_unicode('⠂')],
    '「' => &[decode_unicode('⠐'), decode_unicode('⠦')],
    '」' => &[decode_unicode('⠴'), decode_unicode('⠂')],
    '『' => &[decode_unicode('⠰'), decode_unicode('⠦')],
    '』' => &[decode_unicode('⠴'), decode_unicode('⠆')],
    '/' => &[decode_unicode('⠸'), decode_unicode('⠌')],
    '〈' => &[decode_unicode('⠐'), decode_unicode('⠶')],
    '〉' => &[decode_unicode('⠶'), decode_unicode('⠂')],
    '《' => &[decode_unicode('⠰'), decode_unicode('⠶')],
    '》' => &[decode_unicode('⠶'), decode_unicode('⠆')],
    '―' => &[decode_unicode('⠤'), decode_unicode('⠤')],
    '-' => &[decode_unicode('⠤')],
    '∼' => &[decode_unicode('⠈'), decode_unicode('⠔')],
    '‘' => &[decode_unicode('⠠'), decode_unicode('⠦')],
    '’' => &[decode_unicode('⠴'), decode_unicode('⠄')],
    '○' => &[decode_unicode('⠸'),decode_unicode('⠴'), decode_unicode('⠇')],
    // '×' => &[decode_unicode('⠸'),decode_unicode('⠭'), decode_unicode('⠇')],
    '△' => &[decode_unicode('⠸'),decode_unicode('⠬'), decode_unicode('⠇')],
    '☆' => &[decode_unicode('⠸'),decode_unicode('⠔'), decode_unicode('⠇')],
    '◇' => &[decode_unicode('⠸'),decode_unicode('⠢'), decode_unicode('⠇')],
    '◆' => &[decode_unicode('⠸'),decode_unicode('⠕'), decode_unicode('⠇')],
    '□' => &[decode_unicode('⠸'),decode_unicode('⠶'), decode_unicode('⠇')],
    '•' => &[decode_unicode('⠸'),decode_unicode('⠲')],
    'ː' => &[decode_unicode('⠠'), decode_unicode('⠄')],
    '〃' => &[decode_unicode('⠴'), decode_unicode('⠴')],
    // PDF 제60항 [붙임 1] — 참조 기호 ※ (U+203B).
    '※' => &[decode_unicode('⠸'), decode_unicode('⠔')],
};

static ENGLISH_SYMBOL_MAP: phf::Map<char, &'static [u8]> = phf_map! {
    '(' => &[decode_unicode('⠐'), decode_unicode('⠣')],
    ')' => &[decode_unicode('⠐'), decode_unicode('⠜')],
    ',' => &[decode_unicode('⠂')],
    '-' => &[decode_unicode('⠤')],
    // 제39항 영-한 wrap context의 단어 끝 ':' 영어 점자 (⠒).
    // 일반 영어 단어 끝 ':'은 이 매핑이 있어도 should_render_symbol_as_english가
    // 영어 점자 변환을 결정하므로, 영어 컨텍스트가 끊긴 경우엔 적용되지 않는다.
    ':' => &[decode_unicode('⠒')],
};

pub fn encode_char_symbol_shortcut(text: char) -> Result<&'static [u8], String> {
    if let Some(code) = SHORTCUT_MAP.get(&text) {
        Ok(code)
    } else {
        Err("Invalid symbol character".to_string())
    }
}

pub fn is_symbol_char(text: char) -> bool {
    SHORTCUT_MAP.contains_key(&text)
        || crate::rules::korean::rule_64::is_enclosed_symbol(text)
        || crate::rules::korean::rule_65::is_currency_symbol(text)
        || crate::rules::korean::rule_23::is_historical_letter_symbol(text)
        || crate::rules::korean::rule_25::is_rule_25_symbol(text)
        || crate::rules::korean::rule_31::is_greek_letter(text)
        || crate::rules::korean::rule_68::is_rule_68_symbol(text)
        || crate::rules::korean::rule_69::is_rule_69_symbol(text)
        || crate::rules::korean::rule_70::is_arrow_symbol(text)
        || crate::rules::korean::rule_71::is_rule_71_symbol(text)
        || crate::rules::korean::rule_72::is_rule_72_symbol(text)
}

pub fn encode_english_char_symbol_shortcut(text: char) -> Option<&'static [u8]> {
    ENGLISH_SYMBOL_MAP.get(&text).copied()
}

pub fn is_english_symbol_char(text: char) -> bool {
    ENGLISH_SYMBOL_MAP.contains_key(&text)
}

#[cfg(test)]
mod test {
    use super::*;

    /// `is_symbol_char` — PHF 사전 등록 기호는 true.
    #[rstest::rstest]
    #[case('"')]
    #[case('\'')]
    #[case('~')]
    #[case('…')]
    #[case('!')]
    #[case('.')]
    #[case(',')]
    #[case('?')]
    #[case(':')]
    #[case(';')]
    #[case('_')]
    #[case('*')]
    #[case('(')]
    #[case(')')]
    #[case('{')]
    #[case('}')]
    #[case('①')]
    #[case('ⓐ')]
    #[case('￦')]
    pub fn test_is_symbol_char(#[case] ch: char) {
        assert!(is_symbol_char(ch));
    }

    /// `encode_char_symbol_shortcut` — 기호별 점역 점형 매핑.
    #[rstest::rstest]
    #[case::double_quote('"', vec!['⠦'])]
    #[case::single_quote('\'', vec!['⠠', '⠦'])]
    #[case::tilde('~', vec!['⠈', '⠔'])]
    #[case::horizontal_ellipsis('…', vec!['⠠', '⠠', '⠠'])]
    #[case::midline_ellipsis('⋯', vec!['⠠', '⠠', '⠠'])]
    #[case::exclamation('!', vec!['⠖'])]
    #[case::period('.', vec!['⠲'])]
    #[case::comma(',', vec!['⠐'])]
    #[case::question('?', vec!['⠦'])]
    #[case::colon(':', vec!['⠐', '⠂'])]
    #[case::semicolon(';', vec!['⠰', '⠆'])]
    #[case::underscore('_', vec!['⠤'])]
    #[case::asterisk('*', vec!['⠐', '⠔'])]
    #[case::open_paren('(', vec!['⠦', '⠄'])]
    #[case::close_paren(')', vec!['⠠', '⠴'])]
    pub fn test_encode_char_symbol_shortcut(#[case] ch: char, #[case] expected_unicode: Vec<char>) {
        let expected: Vec<u8> = expected_unicode.into_iter().map(decode_unicode).collect();
        assert_eq!(
            encode_char_symbol_shortcut(ch).unwrap(),
            expected.as_slice()
        );
    }

    #[test]
    fn test_encode_english_char_symbol_shortcut_variants() {
        assert_eq!(
            encode_english_char_symbol_shortcut('(').unwrap(),
            &[decode_unicode('⠐'), decode_unicode('⠣')]
        );
        assert_eq!(
            encode_english_char_symbol_shortcut(')').unwrap(),
            &[decode_unicode('⠐'), decode_unicode('⠜')]
        );
        assert_eq!(encode_english_char_symbol_shortcut('?'), None);
    }
}
