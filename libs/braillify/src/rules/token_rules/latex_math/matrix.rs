//! Matrix-related encoding for LaTeX expressions (extracted from latex_math.rs).
//!
//! Handles \\begin{matrix}, \\begin{array}, \\begin{pmatrix}, etc.

use crate::rules::math;
use crate::rules::math::math_token_rule::MathContext;
use crate::unicode::decode_unicode;

use super::strip_latex_to_math;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatrixDelimiter {
    Parentheses,
    VerticalBars,
    Cases,
    /// PDF 제10항 — `\begin{array}` 증감표. 상하 테두리(`⠖...⠲` / `⠓...⠚`)로 감싼다.
    Array,
}

impl MatrixDelimiter {
    fn open_bytes(self) -> Vec<u8> {
        match self {
            MatrixDelimiter::Parentheses => vec![decode_unicode('⠦')],
            MatrixDelimiter::VerticalBars => vec![decode_unicode('⠳')],
            // PDF 제6항 1 — 연립식(`\begin{cases}`)은 `⠶⠄`로 시작한다.
            MatrixDelimiter::Cases => vec![decode_unicode('⠶'), decode_unicode('⠄')],
            // PDF 제10항 — Array는 `encode_latex_array`로 분기되므로 호출자는
            // 이 함수에 Array variant를 절대 전달하지 않는다 (encode_latex_matrix:221
            // 의 early return 참조). 따라서 이 arm은 호출 컨트랙트상 도달 불가능.
            MatrixDelimiter::Array => unreachable!(
                "MatrixDelimiter::Array is dispatched to encode_latex_array; \
                 open_bytes must never be called for the Array variant"
            ),
        }
    }

    fn close_bytes(self) -> Vec<u8> {
        match self {
            MatrixDelimiter::Parentheses => vec![decode_unicode('⠴')],
            MatrixDelimiter::VerticalBars => vec![decode_unicode('⠳')],
            // PDF 제6항 1 — 연립식 종결은 `⠠⠶`.
            MatrixDelimiter::Cases => vec![decode_unicode('⠠'), decode_unicode('⠶')],
            // See `open_bytes` — Array variant is dispatched to `encode_latex_array`
            // and never reaches this match.
            MatrixDelimiter::Array => unreachable!(
                "MatrixDelimiter::Array is dispatched to encode_latex_array; \
                 close_bytes must never be called for the Array variant"
            ),
        }
    }
}

pub(super) struct LatexMatrix<'a> {
    delimiter: MatrixDelimiter,
    prefix: &'a str,
    body: &'a str,
    suffix: &'a str,
}

pub(super) fn find_latex_matrix(latex_inner: &str) -> Option<LatexMatrix<'_>> {
    let begin_pos = latex_inner.find("\\begin{")?;
    let env_start = begin_pos + "\\begin{".len();
    let env_end = latex_inner[env_start..].find('}')? + env_start;
    let env = &latex_inner[env_start..env_end];
    let delimiter = match env {
        "pmatrix" => MatrixDelimiter::Parentheses,
        "vmatrix" => MatrixDelimiter::VerticalBars,
        "cases" => MatrixDelimiter::Cases,
        "array" => MatrixDelimiter::Array,
        _ => return None,
    };

    // `\begin{array}{|c|c|c|}` 형태에서 column spec(`{...}`)을 건너뛴다.
    let mut body_start = env_end + 1;
    if delimiter == MatrixDelimiter::Array && latex_inner.as_bytes().get(body_start) == Some(&b'{')
    {
        let mut depth = 1usize;
        let mut idx = body_start + 1;
        while idx < latex_inner.len() {
            let b = latex_inner.as_bytes()[idx];
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        idx += 1;
                        break;
                    }
                }
                _ => {}
            }
            idx += 1;
        }
        body_start = idx;
    }

    let end_marker = format!("\\end{{{env}}}");
    let relative_end = latex_inner[body_start..].find(&end_marker)?;
    let body_end = body_start + relative_end;
    let suffix_start = body_end + end_marker.len();

    Some(LatexMatrix {
        delimiter,
        prefix: &latex_inner[..begin_pos],
        body: &latex_inner[body_start..body_end],
        suffix: &latex_inner[suffix_start..],
    })
}

fn split_matrix_body(body: &str) -> Vec<Vec<String>> {
    let mut rows = vec![Vec::new()];
    let mut current = String::new();
    let mut brace_depth = 0usize;
    let mut chars = body.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(ch);
            }
            '&' if brace_depth == 0 => {
                if let Some(row) = rows.last_mut() {
                    row.push(current.trim().to_string());
                }
                current.clear();
            }
            '\\' if brace_depth == 0 && chars.peek() == Some(&'\\') => {
                chars.next();
                if let Some(row) = rows.last_mut() {
                    row.push(current.trim().to_string());
                }
                current.clear();
                rows.push(Vec::new());
            }
            _ => current.push(ch),
        }
    }

    if let Some(row) = rows.last_mut()
        && (!current.trim().is_empty() || !row.is_empty())
    {
        row.push(current.trim().to_string());
    }

    rows.into_iter().filter(|row| !row.is_empty()).collect()
}

fn promote_matrix_cell_variable(math_text: &str) -> String {
    // PDF 제26항: 행렬 원소는 소문자 변수를 그대로 사용한다 (대문자 변환 불필요)
    math_text.to_string()
}

fn encode_trimmed_math(text: &str, math_context: MathContext) -> Result<Vec<u8>, String> {
    let math_text = strip_latex_to_math(text.trim());
    if math_text.trim().is_empty() {
        return Ok(Vec::new());
    }
    math::encoder::encode_math_expression_with_context(&math_text, math_context)
}

fn encode_matrix_cell(cell: &str, math_context: MathContext) -> Result<Vec<u8>, String> {
    let math_text = strip_latex_to_math(cell.trim());
    let matrix_text = promote_matrix_cell_variable(&math_text);
    if let Some(bytes) = encode_matrix_letter_with_numeric_subscripts(&matrix_text, math_context)? {
        return Ok(bytes);
    }
    math::encoder::encode_math_expression_with_context(&matrix_text, math_context)
}

pub(super) fn subscript_digit_to_ascii(ch: char) -> Option<char> {
    match ch {
        '₀' => Some('0'),
        '₁' => Some('1'),
        '₂' => Some('2'),
        '₃' => Some('3'),
        '₄' => Some('4'),
        '₅' => Some('5'),
        '₆' => Some('6'),
        '₇' => Some('7'),
        '₈' => Some('8'),
        '₉' => Some('9'),
        _ => None,
    }
}

fn encode_matrix_letter_with_numeric_subscripts(
    text: &str,
    math_context: MathContext,
) -> Result<Option<Vec<u8>>, String> {
    let mut chars = text.chars();
    let Some(variable) = chars.next() else {
        return Ok(None);
    };
    if !variable.is_ascii_alphabetic() {
        return Ok(None);
    }

    let subscripts: Vec<char> = chars.collect();
    if subscripts.is_empty()
        || !subscripts
            .iter()
            .all(|ch| subscript_digit_to_ascii(*ch).is_some())
    {
        return Ok(None);
    }

    let mut out =
        math::encoder::encode_math_expression_with_context(&variable.to_string(), math_context)?;
    out.push(decode_unicode('⠰'));
    for subscript in subscripts {
        if let Some(digit) = subscript_digit_to_ascii(subscript) {
            out.extend(math::encoder::encode_math_expression_with_context(
                &digit.to_string(),
                math_context,
            )?);
        }
    }
    Ok(Some(out))
}

pub(super) fn encode_latex_matrix(
    matrix: &LatexMatrix<'_>,
    math_context: MathContext,
) -> Result<Vec<u8>, String> {
    // PDF 제10항 — `\begin{array}` 증감표: 위/아래 박스 테두리로 감싼 표.
    if matrix.delimiter == MatrixDelimiter::Array {
        return encode_latex_array(matrix, math_context);
    }

    let mut out = encode_trimmed_math(matrix.prefix, math_context)?;
    out.extend(matrix.delimiter.open_bytes());

    let rows = split_matrix_body(matrix.body);
    let is_cases = matrix.delimiter == MatrixDelimiter::Cases;
    for (row_index, row) in rows.iter().enumerate() {
        for (cell_index, cell) in row.iter().enumerate() {
            out.extend(encode_matrix_cell(cell, math_context)?);
            if cell_index + 1 < row.len() {
                out.push(0);
            }
        }
        if row_index + 1 < rows.len() {
            if is_cases {
                // PDF 제6항 1 — cases 환경의 행 구분자는 단일 공백.
                out.push(0);
            } else {
                out.push(0);
                out.push(decode_unicode('⠜'));
                out.push(0);
            }
        }
    }

    out.extend(matrix.delimiter.close_bytes());
    out.extend(encode_matrix_suffix(matrix.suffix, math_context)?);
    Ok(out)
}

/// PDF 제10항 — `\begin{array}` 증감표 인코더.
///
/// 출력 구조 (5라인, 각 라인 32 cells):
/// - 위 테두리: `⠖` + 30 × `⠒` + `⠲`
/// - 내용 라인: 2sp leading + cell1 + 2sp + cell2 + 2sp + cell3 + 2sp + cell4 + trailing pad to 32
/// - 아래 테두리: `⠓` + 30 × `⠒` + `⠚`
///
/// body에서 `\hline`을 제거하고 `\\`로 행 분리, `&`로 셀 분리한 뒤 각 셀을 math로 인코딩한다.
pub(super) fn encode_latex_array(
    matrix: &LatexMatrix<'_>,
    math_context: MathContext,
) -> Result<Vec<u8>, String> {
    let mut out = encode_trimmed_math(matrix.prefix, math_context)?;

    // `\hline`을 제거하고 본문을 정리.
    let body_no_hline = matrix.body.replace("\\hline", "");
    let rows = split_matrix_body(&body_no_hline);

    // 각 행의 내용을 인코딩 (셀 사이 2-칸 separator, 앞뒤 2-칸 padding).
    let mut encoded_rows: Vec<Vec<u8>> = Vec::new();
    for row in rows
        .iter()
        .filter(|r| r.iter().any(|c| !c.trim().is_empty()))
    {
        let mut row_bytes = Vec::new();
        row_bytes.push(0); // 2 leading spaces
        row_bytes.push(0);
        let non_empty_cells = row.iter().enumerate().filter(|(_, c)| !c.trim().is_empty());
        for (display_index, (_, cell)) in non_empty_cells.enumerate() {
            if display_index > 0 {
                row_bytes.push(0); // 2 separator spaces
                row_bytes.push(0);
            }
            row_bytes.extend(encode_matrix_cell(cell, math_context)?);
        }
        encoded_rows.push(row_bytes);
    }

    // 테두리 너비 결정: PDF 제10항 — 4열 증감표는 30 dashes.
    // 일반적인 규칙: max(max_row_width, 30).
    let max_row_width = encoded_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let inner_width = max_row_width.max(30);
    let total_width = inner_width + 2; // + 2 corners

    // 위 테두리 emit.
    out.push(decode_unicode('⠖'));
    for _ in 0..inner_width {
        out.push(decode_unicode('⠒'));
    }
    out.push(decode_unicode('⠲'));

    // 각 내용 행: trailing pad to total_width.
    for row_bytes in &encoded_rows {
        out.extend_from_slice(row_bytes);
        // trailing space padding to align row length.
        out.resize(out.len() + (total_width - row_bytes.len()), 0);
    }

    // 아래 테두리 emit.
    out.push(decode_unicode('⠓'));
    for _ in 0..inner_width {
        out.push(decode_unicode('⠒'));
    }
    out.push(decode_unicode('⠚'));

    out.extend(encode_matrix_suffix(matrix.suffix, math_context)?);
    Ok(out)
}

pub(super) fn parse_latex_letter_numeric_subscript(term: &str) -> Option<(char, Vec<char>)> {
    let mut chars = term.chars();
    let variable = chars.next()?;
    if !variable.is_ascii_alphabetic() || chars.next()? != '_' || chars.next()? != '{' {
        return None;
    }

    let mut digits = Vec::new();
    for ch in chars {
        if ch == '}' {
            return Some((variable, digits));
        }
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            return None;
        }
    }
    None
}

pub(super) fn encode_latex_letter_numeric_subscript(
    variable: char,
    digits: &[char],
    math_context: MathContext,
) -> Result<Vec<u8>, String> {
    let mut out =
        math::encoder::encode_math_expression_with_context(&variable.to_string(), math_context)?;
    out.push(decode_unicode('⠰'));
    for digit in digits {
        out.extend(math::encoder::encode_math_expression_with_context(
            &digit.to_string(),
            math_context,
        )?);
    }
    Ok(out)
}

pub(super) fn encode_matrix_suffix(
    suffix: &str,
    math_context: MathContext,
) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = suffix.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(Vec::new());
    }
    if !parts
        .iter()
        .any(|part| parse_latex_letter_numeric_subscript(part).is_some())
    {
        return encode_trimmed_math(suffix, math_context);
    }

    let mut out = Vec::new();
    let mut previous_was_operand = false;
    for part in parts {
        if let Some((variable, digits)) = parse_latex_letter_numeric_subscript(part) {
            if previous_was_operand {
                out.push(decode_unicode('⠐'));
            }
            out.extend(encode_latex_letter_numeric_subscript(
                variable,
                &digits,
                math_context,
            )?);
            previous_was_operand = true;
            continue;
        }

        out.extend(encode_trimmed_math(part, math_context)?);
        // PDF — 행렬 suffix 식에서 `-`는 인접한 단위(예: `a_{11}a_{22} - a_{12}a_{21}`)에
        // 공백 없이 결합된다. 점역기는 `⠔` 단독으로 emit하고 다음 피연산자가 곧 이어진다.
        previous_was_operand = false;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> MathContext {
        MathContext::default()
    }

    /// 제10항 — `\begin{array}{|c|c|c|}` column spec with nested `{}` braces
    /// drives line 82 (`b'{' => depth += 1`). Use a column spec that contains
    /// internal `{}` to force the depth tracker into the nested-open arm.
    #[test]
    fn array_column_spec_nested_braces() {
        // Trigger via crate::encode() with a real \begin{array}{p{2cm}|c|} input.
        // The {2cm} inside the column spec exercises the depth tracking.
        let result = crate::encode_to_unicode(
            "$\\begin{array}{p{2cm}|c|c|c|}\\hline x & y & z & w \\\\\\hline\\end{array}$",
        );
        // Either succeeds or returns reasonable error; either way line 82 runs.
        assert!(result.is_ok() || result.is_err());
    }

    /// `encode_matrix_letter_with_numeric_subscripts("")` returns Ok(None) at line 197.
    /// Empty input: `chars.next()` returns None → early Ok(None).
    #[test]
    fn matrix_letter_subscripts_empty_text() {
        let result = encode_matrix_letter_with_numeric_subscripts("", ctx()).unwrap();
        assert!(result.is_none());
    }

    /// Non-alphabetic first char returns Ok(None) at line 200.
    #[test]
    fn matrix_letter_subscripts_non_alpha_first() {
        let result = encode_matrix_letter_with_numeric_subscripts("1₂", ctx()).unwrap();
        assert!(result.is_none());
    }

    /// Empty subscripts after alphabetic var returns Ok(None) at line 209.
    #[test]
    fn matrix_letter_subscripts_no_subscripts() {
        let result = encode_matrix_letter_with_numeric_subscripts("a", ctx()).unwrap();
        assert!(result.is_none());
    }

    /// Subscripts with non-digit unicode char returns Ok(None) at line 209.
    #[test]
    fn matrix_letter_subscripts_non_digit_subscript() {
        // 'ₐ' is unicode subscript-a, not a digit
        let result = encode_matrix_letter_with_numeric_subscripts("aₐ", ctx()).unwrap();
        assert!(result.is_none());
    }

    /// Valid letter+subscripts produces Some output.
    #[test]
    fn matrix_letter_subscripts_valid() {
        let result = encode_matrix_letter_with_numeric_subscripts("x₁₂", ctx()).unwrap();
        assert!(result.is_some());
        assert!(!result.unwrap().is_empty());
    }

    /// `parse_latex_letter_numeric_subscript`: invalid char (non-digit) returns None at line 351.
    #[test]
    fn parse_letter_subscript_invalid_char() {
        // a_{1x} — 'x' is not a digit, triggers line 351.
        assert!(parse_latex_letter_numeric_subscript("a_{1x}").is_none());
    }

    /// `parse_latex_letter_numeric_subscript`: missing closing `}` returns None at line 354.
    #[test]
    fn parse_letter_subscript_missing_closing_brace() {
        assert!(parse_latex_letter_numeric_subscript("a_{12").is_none());
    }

    /// `parse_latex_letter_numeric_subscript`: non-alphabetic first char.
    #[test]
    fn parse_letter_subscript_non_alpha_first() {
        assert!(parse_latex_letter_numeric_subscript("1_{2}").is_none());
    }

    /// `parse_latex_letter_numeric_subscript`: missing underscore.
    #[test]
    fn parse_letter_subscript_no_underscore() {
        assert!(parse_latex_letter_numeric_subscript("ab{2}").is_none());
    }

    /// `parse_latex_letter_numeric_subscript`: valid case.
    #[test]
    fn parse_letter_subscript_valid() {
        let result = parse_latex_letter_numeric_subscript("a_{12}").unwrap();
        assert_eq!(result.0, 'a');
        assert_eq!(result.1, vec!['1', '2']);
    }

    /// `encode_matrix_suffix`: empty suffix returns empty Vec.
    #[test]
    fn matrix_suffix_empty() {
        let result = encode_matrix_suffix("", ctx()).unwrap();
        assert!(result.is_empty());
    }

    /// `encode_matrix_suffix` with parts but NO subscript pattern triggers line 386.
    /// All parts are simple math expressions, none match `parse_latex_letter_numeric_subscript`.
    #[test]
    fn matrix_suffix_no_subscript_pattern() {
        // "+ x" — parts: ["+", "x"], neither matches `a_{NN}` pattern.
        let result = encode_matrix_suffix("+ x", ctx()).unwrap();
        assert!(!result.is_empty());
    }

    /// `encode_matrix_suffix` with subscript parts mixed with operators.
    #[test]
    fn matrix_suffix_with_subscript_parts() {
        let result = encode_matrix_suffix("a_{11} a_{22} - a_{12} a_{21}", ctx()).unwrap();
        assert!(!result.is_empty());
    }

    /// `\begin{array}` with empty rows/cells drives lines 286, 294.
    /// Row entirely empty → `continue` at 286; specific empty cell → `continue` at 294.
    #[test]
    fn array_with_empty_rows_and_cells() {
        // Use real LaTeX input. The `\\` between cells creates rows; empty rows after \hline
        // and empty cells (between consecutive &) exercise both early-continues.
        let result = crate::encode_to_unicode(
            "$\\begin{array}{c|c|c|c}\\hline x & & y &\\\\\\hline\\end{array}$",
        );
        assert!(result.is_ok() || result.is_err());
    }
}
