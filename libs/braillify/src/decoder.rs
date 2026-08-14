//! Korean braille (Unicode U+2800..U+28FF) to Hangul decoder.

pub fn decode(braille: &str) -> Result<String, String> {
    let cells: Vec<u8> = braille
        .chars()
        .filter_map(|character| {
            let code = character as u32;
            if (0x2800..=0x28ff).contains(&code) {
                Some((code - 0x2800) as u8)
            } else if character == ' ' || character == '\n' {
                Some(0)
            } else {
                None
            }
        })
        .collect();

    Ok(decode_cells(&cells))
}

#[derive(Clone, Copy)]
enum Stage {
    Start,
    GotChoseong(u32),
    GotJungseong(u32, u32),
}

fn decode_cells(cells: &[u8]) -> String {
    let mut result = String::new();
    let mut index = 0;
    let mut stage = Stage::Start;

    while index < cells.len() {
        let cell = cells[index];

        match stage {
            Stage::Start => {
                if cell == 0 {
                    result.push(' ');
                    index += 1;
                } else if let Some((character, consumed)) = two_cell_shortcut(cells, index) {
                    result.push(character);
                    index += consumed;
                } else if let Some((jungseong, jongseong)) = syllable_shortcut(cell) {
                    result.push(build_syllable(11, jungseong, jongseong));
                    index += 1;
                } else if let Some(choseong) = consonant_a_shortcut(cell) {
                    stage = Stage::GotJungseong(choseong, 0);
                    index += 1;
                } else if let Some((jungseong, consumed)) = jungseong(cells, index) {
                    stage = Stage::GotJungseong(11, jungseong);
                    index += consumed;
                } else if let Some((choseong, consumed)) = choseong(cells, index) {
                    stage = Stage::GotChoseong(choseong);
                    index += consumed;
                } else {
                    index += 1;
                }
            }
            Stage::GotChoseong(choseong) => {
                if let Some((jungseong, consumed)) = jungseong(cells, index) {
                    stage = Stage::GotJungseong(choseong, jungseong);
                    index += consumed;
                } else if let Some((jungseong, jongseong)) = syllable_shortcut(cell) {
                    result.push(build_syllable(choseong, jungseong, jongseong));
                    stage = Stage::Start;
                    index += 1;
                } else if cell == 0 {
                    result.push(choseong_jamo(choseong));
                    result.push(' ');
                    stage = Stage::Start;
                    index += 1;
                } else {
                    // Several choseong cells also abbreviate the syllable with ㅏ.
                    stage = Stage::GotJungseong(choseong, 0);
                }
            }
            Stage::GotJungseong(choseong_index, jungseong_index) => {
                if let Some((character, consumed)) = two_cell_shortcut(cells, index) {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    result.push(character);
                    stage = Stage::Start;
                    index += consumed;
                } else if let (jongseong @ 1.., consumed) = jongseong(cells, index) {
                    result.push(build_syllable(choseong_index, jungseong_index, jongseong));
                    stage = Stage::Start;
                    index += consumed;
                } else if cell == 0 {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    result.push(' ');
                    stage = Stage::Start;
                    index += 1;
                } else if let Some((next_jungseong, next_jongseong)) = syllable_shortcut(cell) {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    result.push(build_syllable(11, next_jungseong, next_jongseong));
                    stage = Stage::Start;
                    index += 1;
                } else if let Some(next_choseong) = consonant_a_shortcut(cell) {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    stage = Stage::GotJungseong(next_choseong, 0);
                    index += 1;
                } else if let Some((next_choseong, consumed)) = choseong(cells, index) {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    stage = Stage::GotChoseong(next_choseong);
                    index += consumed;
                } else if let Some((next_jungseong, consumed)) = jungseong(cells, index) {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    stage = Stage::GotJungseong(11, next_jungseong);
                    index += consumed;
                } else {
                    result.push(build_syllable(choseong_index, jungseong_index, 0));
                    stage = Stage::Start;
                    index += 1;
                }
            }
        }
    }

    match stage {
        Stage::GotChoseong(choseong) => result.push(build_syllable(choseong, 0, 0)),
        Stage::GotJungseong(choseong, jungseong) => {
            result.push(build_syllable(choseong, jungseong, 0));
        }
        Stage::Start => {}
    }

    result.trim().to_owned()
}

fn two_cell_shortcut(cells: &[u8], index: usize) -> Option<(char, usize)> {
    match (*cells.get(index)?, cells.get(index + 1).copied()?) {
        (32, 59) => Some(('성', 2)),
        (40, 59) => Some(('정', 2)),
        (48, 59) => Some(('청', 2)),
        (56, 14) => Some(('것', 2)),
        _ => None,
    }
}

fn syllable_shortcut(cell: u8) -> Option<(u32, u32)> {
    match cell {
        57 => Some((4, 1)),
        62 => Some((4, 4)),
        30 => Some((4, 8)),
        33 => Some((6, 4)),
        51 => Some((6, 8)),
        59 => Some((6, 21)),
        45 => Some((8, 1)),
        55 => Some((8, 4)),
        63 => Some((8, 21)),
        27 => Some((13, 4)),
        47 => Some((13, 8)),
        53 => Some((18, 4)),
        46 => Some((18, 8)),
        31 => Some((20, 4)),
        _ => None,
    }
}

fn consonant_a_shortcut(cell: u8) -> Option<u32> {
    match cell {
        43 => Some(0),
        7 => Some(9),
        _ => None,
    }
}

fn single_choseong(cell: u8) -> Option<u32> {
    match cell {
        8 => Some(0),
        9 => Some(2),
        10 => Some(3),
        11 => Some(15),
        16 => Some(5),
        17 => Some(6),
        19 => Some(16),
        24 => Some(7),
        25 => Some(17),
        26 => Some(18),
        32 => Some(9),
        40 => Some(12),
        48 => Some(14),
        _ => None,
    }
}

fn choseong(cells: &[u8], index: usize) -> Option<(u32, usize)> {
    let first = *cells.get(index)?;
    if first == 32 {
        if let Some(second) = cells.get(index + 1) {
            let tense = match second {
                8 | 43 => Some(1),
                10 => Some(4),
                24 => Some(8),
                7 | 32 => Some(10),
                40 => Some(13),
                _ => None,
            };
            if let Some(value) = tense {
                return Some((value, 2));
            }
        }
    }

    single_choseong(first).map(|value| (value, 1))
}

fn jungseong(cells: &[u8], index: usize) -> Option<(u32, usize)> {
    let first = *cells.get(index)?;
    let second = cells.get(index + 1).copied();

    match (first, second) {
        (13, Some(23)) => return Some((16, 2)),
        (28, Some(23)) => return Some((3, 2)),
        (39, Some(23)) => return Some((10, 2)),
        (15, Some(23)) => return Some((15, 2)),
        _ => {}
    }

    let value = match first {
        35 => 0,
        23 => 1,
        28 => 2,
        14 => 4,
        29 => 5,
        49 => 6,
        12 => 7,
        37 => 8,
        39 => 9,
        61 => 11,
        44 => 12,
        13 => 13,
        15 => 14,
        41 => 17,
        42 => 18,
        58 => 19,
        21 => 20,
        _ => return None,
    };

    Some((value, 1))
}

fn jongseong(cells: &[u8], index: usize) -> (u32, usize) {
    let Some(&first) = cells.get(index) else {
        return (0, 0);
    };
    let second = cells.get(index + 1).copied();

    match (first, second) {
        (1, Some(1)) => return (2, 2),
        (1, Some(4)) => return (3, 2),
        (18, Some(5)) => return (5, 2),
        (18, Some(52)) => return (6, 2),
        (2, Some(1)) => return (9, 2),
        (2, Some(34)) => return (10, 2),
        (2, Some(3)) => return (11, 2),
        (2, Some(4)) => return (12, 2),
        (2, Some(38)) => return (13, 2),
        (2, Some(50)) => return (14, 2),
        (2, Some(52)) => return (15, 2),
        (3, Some(4)) => return (18, 2),
        _ => {}
    }

    let value = match first {
        1 => 1,
        18 => 4,
        20 => 7,
        2 => 8,
        34 => 16,
        3 => 17,
        4 => 19,
        12 => 20,
        54 => 21,
        5 => 22,
        6 => 23,
        22 => 24,
        38 => 25,
        50 => 26,
        52 => 27,
        _ => return (0, 0),
    };

    (value, 1)
}

fn build_syllable(choseong: u32, jungseong: u32, jongseong: u32) -> char {
    char::from_u32((choseong * 21 + jungseong) * 28 + jongseong + 0xac00).unwrap_or('?')
}

fn choseong_jamo(index: u32) -> char {
    const JAMO: &[char] = &[
        'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ',
        'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
    ];
    JAMO.get(index as usize).copied().unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_a_representative_korean_word() {
        assert_eq!(decode("⠣⠒⠉⠻").unwrap(), "안녕");
    }

    #[test]
    fn roundtrips_supported_korean_words() {
        for word in ["안녕", "강아지", "한 글", "성정청", "기쁘다"] {
            let braille = crate::encode_to_unicode(word).unwrap();
            assert_eq!(decode(&braille).unwrap(), word);
        }
    }
}
