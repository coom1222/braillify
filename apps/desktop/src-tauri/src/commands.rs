const EMPTY_INPUT_MESSAGE: &str = "점역할 내용을 입력해 주세요.";

#[tauri::command]
pub(crate) fn translate_to_unicode(input: String) -> Result<String, String> {
    if input.trim().is_empty() {
        return Err(EMPTY_INPUT_MESSAGE.to_owned());
    }

    braillify::encode_to_unicode(&input)
        .map_err(|error| format!("점역할 수 없는 내용입니다: {error}"))
}

#[cfg(test)]
mod tests {
    use super::translate_to_unicode;

    enum Expected {
        ErrorContaining(&'static str),
        Success,
    }

    #[rstest::rstest]
    #[case::normal_korean("안녕", Expected::Success)]
    #[case::line_break("안녕\n안녕", Expected::Success)]
    #[case::empty("", Expected::ErrorContaining("입력"))]
    #[case::whitespace_only(" \n\t", Expected::ErrorContaining("입력"))]
    #[case::unsupported_emoji("😀", Expected::ErrorContaining("점역할 수 없는"))]
    fn translates_or_rejects_expected_inputs(#[case] input: &str, #[case] expected: Expected) {
        let actual = translate_to_unicode(input.to_owned());

        match expected {
            Expected::Success => {
                assert_eq!(actual, braillify::encode_to_unicode(input));
                assert!(!actual.unwrap().is_empty());
            }
            Expected::ErrorContaining(message) => {
                assert!(actual.unwrap_err().contains(message));
            }
        }
    }

    #[test]
    fn translates_long_input_with_the_same_core_algorithm() {
        let input = "안녕 ".repeat(1_000);
        let actual = translate_to_unicode(input.clone());

        assert_eq!(actual, braillify::encode_to_unicode(&input));
        assert!(!actual.unwrap().is_empty());
    }
}
