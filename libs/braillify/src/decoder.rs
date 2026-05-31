/// Simple Korean braille decoder (점자 → 한글).
/// Handles basic Korean syllables, char shortcuts (가나다…), and abbreviated
/// syllable shortcuts (을 억 언 …). Numbers, English, and word shortcuts are
/// not yet handled.
pub fn decode(braille: &str) -> Result<String, String> {
    let bytes: Vec<u8> = braille
        .chars()
        .filter_map(|c| {
            let code = c as u32;
            if code >= 0x2800 && code <= 0x28FF {
                Some((code - 0x2800) as u8)
            } else if c == ' ' || c == '\n' {
                Some(0)
            } else {
                None
            }
        })
        .collect();

    decode_bytes(&bytes)
}

fn decode_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut result = String::new();
    let mut i = 0;

    enum Stage {
        Start,
        GotCho(u32),
        GotJung(u32, u32), // (cho_idx, jung_idx)
    }

    let mut stage = Stage::Start;

    while i < bytes.len() {
        let b = bytes[i];

        match stage {
            // ─── Start: no pending syllable ──────────────────────────────
            Stage::Start => {
                if b == 0 {
                    result.push(' ');
                    i += 1;
                } else if let Some((ch, len)) = try_two_byte_shortcut(bytes, i) {
                    result.push(ch);
                    i += len;
                } else if let Some((jung, jong)) = try_abbreviated_shortcut(b) {
                    // standalone 을/억/언/… → ㅇ초성 + jung + jong
                    result.push(build_syllable(11, jung, jong));
                    i += 1;
                } else if let Some(cho) = try_consonant_a_shortcut(b) {
                    // 가(43) or 사(7) — need to collect possible jongseong
                    stage = Stage::GotJung(cho, 0); // jung=0 = ㅏ
                    i += 1;
                } else if let Some((jung, consumed)) = try_jungseong(bytes, i) {
                    stage = Stage::GotJung(11, jung); // silent ㅇ
                    i += consumed;
                } else if let Some((cho, consumed)) = try_choseong_with_double(bytes, i) {
                    stage = Stage::GotCho(cho);
                    i += consumed;
                } else {
                    i += 1; // skip unknown byte
                }
            }

            // ─── GotCho: have initial consonant, waiting for vowel ────────
            Stage::GotCho(cho) => {
                if let Some((jung, consumed)) = try_jungseong(bytes, i) {
                    stage = Stage::GotJung(cho, jung);
                    i += consumed;
                } else if let Some((jung, jong)) = try_abbreviated_shortcut(b) {
                    // e.g. 글 = ㄱ(8) + 을(46) → cho + ㅡ + ㄹ
                    result.push(build_syllable(cho, jung, jong));
                    stage = Stage::Start;
                    i += 1;
                } else if b == 0 {
                    // consonant followed immediately by space → standalone jamo
                    result.push(cho_idx_to_jamo(cho));
                    result.push(' ');
                    stage = Stage::Start;
                    i += 1;
                } else {
                    // No vowel followed — this choseong byte doubles as a
                    // 나/다/마/바/자/카/타/파/하 shortcut (implicit ㅏ vowel).
                    // Don't consume the current byte; let GotJung handle it.
                    stage = Stage::GotJung(cho, 0); // jung=0 = ㅏ
                }
            }

            // ─── GotJung: have cho+jung, looking for jongseong ───────────
            Stage::GotJung(cho, jung) => {
                // Priority 1: two-byte shortcuts (성 정 청 것) — check before jongseong
                // because their first bytes (32,40,48,56) can also start jongseong sequences.
                if let Some((ch, len)) = try_two_byte_shortcut(bytes, i) {
                    result.push(build_syllable(cho, jung, 0));
                    result.push(ch);
                    stage = Stage::Start;
                    i += len;
                }
                // Priority 2: jongseong
                else if let (jong @ 1.., len) = try_jongseong(bytes, i) {
                    result.push(build_syllable(cho, jung, jong));
                    stage = Stage::Start;
                    i += len;
                }
                // Priority 3: space
                else if b == 0 {
                    result.push(build_syllable(cho, jung, 0));
                    result.push(' ');
                    stage = Stage::Start;
                    i += 1;
                }
                // Priority 4: abbreviated shortcut byte starts a new syllable
                // (e.g. "나을" — emit 나, then 을 as ㅇ+ㅡ+ㄹ)
                else if let Some((jung2, jong2)) = try_abbreviated_shortcut(b) {
                    result.push(build_syllable(cho, jung, 0));
                    result.push(build_syllable(11, jung2, jong2));
                    stage = Stage::Start;
                    i += 1;
                }
                // Priority 5: 가(43) or 사(7) shortcut starts a new syllable
                else if let Some(cho2) = try_consonant_a_shortcut(b) {
                    result.push(build_syllable(cho, jung, 0));
                    stage = Stage::GotJung(cho2, 0);
                    i += 1;
                }
                // Priority 6: choseong starts next syllable (incl. tense consonants)
                else if let Some((new_cho, consumed)) = try_choseong_with_double(bytes, i) {
                    result.push(build_syllable(cho, jung, 0));
                    stage = Stage::GotCho(new_cho);
                    i += consumed;
                }
                // Priority 7: consecutive vowel → emit current, silent ㅇ + new vowel
                else if let Some((new_jung, consumed)) = try_jungseong(bytes, i) {
                    result.push(build_syllable(cho, jung, 0));
                    stage = Stage::GotJung(11, new_jung);
                    i += consumed;
                }
                // Unknown byte
                else {
                    result.push(build_syllable(cho, jung, 0));
                    stage = Stage::Start;
                    i += 1;
                }
            }
        }
    }

    // Flush remaining state
    match stage {
        Stage::GotCho(cho) => result.push(build_syllable(cho, 0, 0)), // implicit ㅏ shortcut
        Stage::GotJung(cho, jung) => result.push(build_syllable(cho, jung, 0)),
        Stage::Start => {}
    }

    Ok(result.trim().to_string())
}

// ── Two-byte shortcuts ─────────────────────────────────────────────────────
// 성/정/청/것 are conventional pairings — NOT phonetically composed.
fn try_two_byte_shortcut(bytes: &[u8], i: usize) -> Option<(char, usize)> {
    let b0 = *bytes.get(i)?;
    let b1 = bytes.get(i + 1).copied()?;
    match (b0, b1) {
        (32, 59) => Some(('성', 2)), // ⠠⠻
        (40, 59) => Some(('정', 2)), // ⠨⠻
        (48, 59) => Some(('청', 2)), // ⠰⠻
        (56, 14) => Some(('것', 2)), // ⠸⠎
        _ => None,
    }
}

// ── Abbreviated syllable shortcuts ────────────────────────────────────────
// These bytes represent a (jung, jong) pair when they appear:
//   • after a choseong → GotCho uses them to complete the syllable
//   • in Start/GotJung → emit ㅇ(11)+jung+jong as standalone
//
// Unicode TIndex (jongseong position in syllable formula):
//   ㄱ=1 ㄴ=4 ㄷ=7 ㄹ=8 ㅁ=16 ㅂ=17 ㅅ=19 ㅆ=20 ㅇ=21 ㅈ=22 ㅊ=23 ㅋ=24 ㅌ=25 ㅍ=26 ㅎ=27
fn try_abbreviated_shortcut(b: u8) -> Option<(u32, u32)> {
    match b {
        57 => Some((4,  1)),  // 억: ㅓ + ㄱ
        62 => Some((4,  4)),  // 언: ㅓ + ㄴ
        30 => Some((4,  8)),  // 얼: ㅓ + ㄹ
        33 => Some((6,  4)),  // 연: ㅕ + ㄴ
        51 => Some((6,  8)),  // 열: ㅕ + ㄹ
        59 => Some((6,  21)), // 영: ㅕ + ㅇ
        45 => Some((8,  1)),  // 옥: ㅗ + ㄱ
        55 => Some((8,  4)),  // 온: ㅗ + ㄴ
        63 => Some((8,  21)), // 옹: ㅗ + ㅇ
        27 => Some((13, 4)),  // 운: ㅜ + ㄴ
        47 => Some((13, 8)),  // 울: ㅜ + ㄹ
        53 => Some((18, 4)),  // 은: ㅡ + ㄴ
        46 => Some((18, 8)),  // 을: ㅡ + ㄹ
        31 => Some((20, 4)),  // 인: ㅣ + ㄴ
        _  => None,
    }
}

// ── Consonant-a shortcuts (가, 사) ────────────────────────────────────────
// These bytes encode a specific consonant + implicit ㅏ vowel,
// but unlike the shared-byte shortcuts (나=9=ㄴ초성) these bytes
// are NOT in the choseong map, so we need a separate handler.
fn try_consonant_a_shortcut(b: u8) -> Option<u32> {
    match b {
        43 => Some(0),  // 가: ㄱ(0) + ㅏ
        7  => Some(9),  // 사: ㅅ(9) + ㅏ
        _  => None,
    }
}

// ── Choseong ─────────────────────────────────────────────────────────────
// Unicode choseong indices:
// ㄱ=0 ㄲ=1 ㄴ=2 ㄷ=3 ㄸ=4 ㄹ=5 ㅁ=6 ㅂ=7 ㅃ=8 ㅅ=9 ㅆ=10
// ㅇ=11 ㅈ=12 ㅉ=13 ㅊ=14 ㅋ=15 ㅌ=16 ㅍ=17 ㅎ=18
fn try_choseong(b: u8) -> Option<u32> {
    match b {
        8  => Some(0),  // ㄱ
        9  => Some(2),  // ㄴ  (also 나 shortcut when not followed by jungseong)
        10 => Some(3),  // ㄷ  (also 다 shortcut)
        11 => Some(15), // ㅋ  (also 카 shortcut)
        16 => Some(5),  // ㄹ
        17 => Some(6),  // ㅁ  (also 마 shortcut)
        19 => Some(16), // ㅌ  (also 타 shortcut)
        24 => Some(7),  // ㅂ  (also 바 shortcut)
        25 => Some(17), // ㅍ  (also 파 shortcut)
        26 => Some(18), // ㅎ  (also 하 shortcut)
        32 => Some(9),  // ㅅ  (also 사-related, but ⠠ 사=7)
        40 => Some(12), // ㅈ  (also 자 shortcut)
        48 => Some(14), // ㅊ
        _  => None,
    }
}

// ── Double (tense) choseong — 제2항 된소리 ───────────────────────────────
// 된소리표 ⠠ (byte 32) + base consonant byte → tense choseong.
//   32 + 8  → ㄲ (idx 1)
//   32 + 10 → ㄸ (idx 4)
//   32 + 24 → ㅃ (idx 8)
//   32 + 32 → ㅆ (idx 10)
//   32 + 40 → ㅉ (idx 13)
// Falls back to try_choseong (single byte, ㅅ) if not a double pair.
fn try_choseong_with_double(bytes: &[u8], i: usize) -> Option<(u32, usize)> {
    let b0 = *bytes.get(i)?;
    if b0 == 32 {
        if let Some(&b1) = bytes.get(i + 1) {
            // 된소리표(32) 뒤에 오는 바이트 패턴:
            //  ㅏ 모음: 된소리표 + 약자 바이트 (가=43, 사=7, 나/다/바/자는 공유)
            //  기타 모음: 된소리표 + 초성 바이트 (ㄱ=8, ㅅ=32) + 별도 중성
            // 예) 까=[32,43]  꺼=[32,8,14]  싸=[32,7]  쏘=[32,32,37]
            let double_idx = match b1 {
                8  | 43 => Some(1),  // ㄲ: 8=ㄱ초성(꺼·끼…), 43=가약자(까)
                10      => Some(4),  // ㄸ: 공유바이트 (따·뚜… 모두)
                24      => Some(8),  // ㅃ: 공유바이트 (빠·뿌… 모두)
                7  | 32 => Some(10), // ㅆ: 7=사약자(싸), 32=ㅅ초성(쏘·씩…)
                40      => Some(13), // ㅉ: 공유바이트 (짜·쭈… 모두)
                _       => None,
            };
            if let Some(idx) = double_idx {
                return Some((idx, 2));
            }
        }
    }
    try_choseong(b0).map(|idx| (idx, 1))
}

// ── Jungseong ────────────────────────────────────────────────────────────
// Unicode jungseong indices:
// ㅏ=0 ㅐ=1 ㅑ=2 ㅒ=3 ㅓ=4 ㅔ=5 ㅕ=6 ㅖ=7 ㅗ=8 ㅘ=9 ㅙ=10 ㅚ=11 ㅛ=12
// ㅜ=13 ㅝ=14 ㅞ=15 ㅟ=16 ㅠ=17 ㅡ=18 ㅢ=19 ㅣ=20
fn try_jungseong(bytes: &[u8], i: usize) -> Option<(u32, usize)> {
    let b = *bytes.get(i)?;
    let b2 = bytes.get(i + 1).copied();

    // Compound vowels (2 bytes) checked first
    match (b, b2) {
        (13, Some(23)) => return Some((16, 2)), // ㅟ  [⠍⠗]
        (28, Some(23)) => return Some((3,  2)), // ㅒ  [⠜⠗]
        (39, Some(23)) => return Some((10, 2)), // ㅙ  [⠧⠗]
        (15, Some(23)) => return Some((15, 2)), // ㅞ  [⠏⠗]
        _ => {}
    }

    let idx = match b {
        35 => 0,  // ㅏ
        23 => 1,  // ㅐ
        28 => 2,  // ㅑ
        14 => 4,  // ㅓ
        29 => 5,  // ㅔ
        49 => 6,  // ㅕ
        12 => 7,  // ㅖ
        37 => 8,  // ㅗ
        39 => 9,  // ㅘ
        61 => 11, // ㅚ
        44 => 12, // ㅛ
        13 => 13, // ㅜ
        15 => 14, // ㅝ
        41 => 17, // ㅠ
        42 => 18, // ㅡ
        58 => 19, // ㅢ
        21 => 20, // ㅣ
        _  => return None,
    };
    Some((idx, 1))
}

// ── Jongseong ────────────────────────────────────────────────────────────
// Unicode jongseong TIndex (0 = none):
// ㄱ=1 ㄲ=2 ㄳ=3 ㄴ=4 ㄵ=5 ㄶ=6 ㄷ=7 ㄹ=8
// ㄺ=9 ㄻ=10 ㄼ=11 ㄽ=12 ㄾ=13 ㄿ=14 ㅀ=15 ㅁ=16 ㅂ=17 ㅄ=18
// ㅅ=19 ㅆ=20 ㅇ=21 ㅈ=22 ㅊ=23 ㅋ=24 ㅌ=25 ㅍ=26 ㅎ=27
fn try_jongseong(bytes: &[u8], i: usize) -> (u32, usize) {
    let b = match bytes.get(i) {
        Some(&v) => v,
        None => return (0, 0),
    };
    let b2 = bytes.get(i + 1).copied();

    // Compound jongseong (2 bytes) checked first
    match (b, b2) {
        (1,  Some(1))  => return (2,  2), // ㄲ
        (1,  Some(4))  => return (3,  2), // ㄳ
        (18, Some(5))  => return (5,  2), // ㄵ
        (18, Some(52)) => return (6,  2), // ㄶ
        (2,  Some(1))  => return (9,  2), // ㄺ
        (2,  Some(34)) => return (10, 2), // ㄻ
        (2,  Some(3))  => return (11, 2), // ㄼ
        (2,  Some(4))  => return (12, 2), // ㄽ
        (2,  Some(38)) => return (13, 2), // ㄾ
        (2,  Some(50)) => return (14, 2), // ㄿ
        (2,  Some(52)) => return (15, 2), // ㅀ
        (3,  Some(4))  => return (18, 2), // ㅄ
        _ => {}
    }

    let idx = match b {
        1  => 1,  // ㄱ
        18 => 4,  // ㄴ
        20 => 7,  // ㄷ
        2  => 8,  // ㄹ
        34 => 16, // ㅁ
        3  => 17, // ㅂ
        4  => 19, // ㅅ
        12 => 20, // ㅆ
        54 => 21, // ㅇ
        5  => 22, // ㅈ
        6  => 23, // ㅊ
        22 => 24, // ㅋ
        38 => 25, // ㅌ
        50 => 26, // ㅍ
        52 => 27, // ㅎ
        _  => return (0, 0),
    };
    (idx, 1)
}

// ── Syllable builder ─────────────────────────────────────────────────────

fn build_syllable(cho: u32, jung: u32, jong: u32) -> char {
    let code = (cho * 21 + jung) * 28 + jong + 0xAC00;
    char::from_u32(code).unwrap_or('?')
}

fn cho_idx_to_jamo(idx: u32) -> char {
    const JAMO: &[char] = &[
        'ㄱ','ㄲ','ㄴ','ㄷ','ㄸ','ㄹ','ㅁ','ㅂ','ㅃ',
        'ㅅ','ㅆ','ㅇ','ㅈ','ㅉ','ㅊ','ㅋ','ㅌ','ㅍ','ㅎ',
    ];
    JAMO.get(idx as usize).copied().unwrap_or('?')
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(text: &str) -> String {
        let encoded = crate::encode_to_unicode(text).expect("encode failed");
        decode(&encoded).expect("decode failed")
    }

    #[test]
    fn test_geo_ri() {
        assert_eq!(decode("⠈⠎⠐⠕").unwrap(), "거리");
    }

    #[test]
    fn test_gang_a_ji() {
        assert_eq!(roundtrip("강아지"), "강아지");
    }

    #[test]
    fn test_han_geul() {
        assert_eq!(roundtrip("한 글"), "한 글");
    }

    #[test]
    fn test_na_ga_da() {
        assert_eq!(roundtrip("나가다"), "나가다");
    }

    #[test]
    fn test_abbreviated_syllables() {
        // 을 억 언 얼 … standalone
        for word in &["을", "억", "언", "얼", "연", "열", "영", "옥", "온", "옹",
                      "운", "울", "은", "인"] {
            assert_eq!(&roundtrip(word), word, "failed on: {word}");
        }
    }

    #[test]
    fn test_seong_jeong_cheong() {
        assert_eq!(roundtrip("성정청"), "성정청");
    }

    #[test]
    fn test_geo_seong() {
        assert_eq!(roundtrip("거성"), "거성");
    }

    fn to_bytes(s: &str) -> Vec<u8> {
        s.chars().filter_map(|c| {
            let code = c as u32;
            if code >= 0x2800 { Some((code - 0x2800) as u8) } else { None }
        }).collect()
    }

    #[test]
    fn test_tense_consonants() {
        // ㅏ 모음: 된소리표 + 약자/공유 바이트 (compact form)
        for word in &["까", "따", "빠", "싸", "짜"] {
            assert_eq!(&roundtrip(word), word, "failed on: {word}");
        }
        // ㅏ 외 모음: 된소리표 + 초성 바이트 + 별도 중성
        assert_eq!(roundtrip("꺼"), "꺼"); // ㄲ+ㅓ → [32,8,14]
        assert_eq!(roundtrip("뚜"), "뚜"); // ㄸ+ㅜ → [32,10,13]
        assert_eq!(roundtrip("쏘"), "쏘"); // ㅆ+ㅗ → [32,32,37]
        // 단어 안에서도 동작
        assert_eq!(roundtrip("까다"), "까다");
        assert_eq!(roundtrip("짜다"), "짜다");
    }
}
