//! Reproducible NIKL Korean–Korean Braille Parallel Corpus 2025 v1.0 analysis.
//!
//! Run from the workspace root:
//! `cargo run --release -p braillify --example nikl_corpus_analyze`
//!
//! This is an offline evaluation tool. It deliberately deserializes only `input` and
//! `unicode`; the read-only competitor fields `world` and `jeomsarang` are neither loaded nor
//! compared.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Deserialize)]
struct CorpusCase {
    input: String,
    unicode: String,
}

#[derive(Clone)]
struct LocatedCase {
    shard: String,
    index: usize,
    case: CorpusCase,
}

#[derive(Clone)]
struct EncodedCase {
    located: LocatedCase,
    actual: Result<String, String>,
    nfc_actual: Option<Result<String, String>>,
    nfkc_actual: Option<Result<String, String>>,
    singleton_unsupported_characters: Vec<char>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum PrimaryClass {
    Exact,
    ImplementationDefect,
    UnsupportedCharacterReview,
    UnclassifiedEncodingErrorReview,
    CorpusSuspect,
    ComparisonMethod,
    PendingRuleReview,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Reason {
    Exact,
    ConflictingDuplicateReference,
    Rule34RomanIndicatorBeforeOpeningParenthesis,
    BrailleWhitespaceEquivalent,
    NfcInputEquivalent,
    NfkcInputEquivalent,
    RomanIndicatorAfterCapitalIndicator,
    UnsupportedCharacterReview,
    UnclassifiedEncodingErrorReview,
    ForeignTextRuleReview,
    NumberRuleReview,
    PunctuationRuleReview,
    KoreanRuleReview,
}

#[derive(Debug, Serialize)]
struct Sample {
    shard: String,
    index: usize,
    input: String,
    expected_excerpt: String,
    actual_excerpt: String,
    error: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct ShardStats {
    total: usize,
    exact: usize,
}

#[derive(Debug, Serialize)]
struct ErrorCharacterStats {
    cases: usize,
    nfkc: String,
    family: &'static str,
}

#[derive(Debug, Serialize)]
struct Rule36ComplexErrorSample {
    shard: String,
    index: usize,
    input: String,
    other_unsupported_characters: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct Rule36TransitionAudit {
    presentation_cases: usize,
    observed_transitions: BTreeMap<String, usize>,
    remaining_complex_errors: usize,
    remaining_complex_error_samples: Vec<Rule36ComplexErrorSample>,
}

#[derive(Debug, Serialize)]
struct UnclassifiedEncodingErrorSample {
    shard: String,
    index: usize,
    input: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct MultipleSingletonErrorSample {
    shard: String,
    index: usize,
    input: String,
    unsupported_characters: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct EncodingErrorAudit {
    raw_total: usize,
    resolved_by_comparison_method: usize,
    excluded_as_corpus_suspect: usize,
    unresolved_review_total: usize,
    explained_by_singleton_unsupported: usize,
    multiple_singleton_unsupported: usize,
    multiple_singleton_samples: Vec<MultipleSingletonErrorSample>,
    unclassified_without_singleton: usize,
    unclassified_samples: Vec<UnclassifiedEncodingErrorSample>,
}

#[derive(Clone, Debug, Serialize)]
struct PendingRuleReviewClusterSample {
    shard: String,
    index: usize,
    input: String,
    expected_excerpt: String,
    actual_excerpt: String,
    first_difference_cell: Option<usize>,
    error: Option<String>,
    primary_class: String,
    reason: String,
}

#[derive(Debug, Default, Serialize)]
struct PendingRuleReviewClusterStats {
    candidates: usize,
    exact: usize,
    mismatch: usize,
    conflicting_reference_cases: usize,
    output_signature_mismatches_evaluated: usize,
    first_difference_in_output_signature: usize,
    first_difference_in_output_signature_transitions: BTreeMap<String, usize>,
    mismatch_primary_classes: BTreeMap<String, usize>,
    samples: BTreeMap<String, Vec<PendingRuleReviewClusterSample>>,
}

#[derive(Debug, Default, Serialize)]
struct FirstDifferenceTransitionStats {
    cases: usize,
    samples: Vec<PendingRuleReviewClusterSample>,
}

#[derive(Debug, Serialize)]
struct AnalysisReport {
    corpus: &'static str,
    total: usize,
    exact: usize,
    mismatch: usize,
    exact_percent: f64,
    duplicate_inputs: usize,
    conflicting_duplicate_inputs: usize,
    primary_classes: BTreeMap<String, usize>,
    reasons: BTreeMap<String, usize>,
    encoding_error_messages: BTreeMap<String, usize>,
    encoding_error_families: BTreeMap<String, usize>,
    singleton_error_characters: BTreeMap<String, ErrorCharacterStats>,
    encoding_error_audit: EncodingErrorAudit,
    rule_36_transition_audit: Rule36TransitionAudit,
    // Cross-cutting input cohorts; only members whose existing primary class
    // is PendingRuleReview are pending-rule-review subclusters.
    pending_rule_review_clusters: BTreeMap<String, PendingRuleReviewClusterStats>,
    pending_first_difference_cell_transitions: BTreeMap<String, FirstDifferenceTransitionStats>,
    pending_first_difference_transitions_after_localized_cohorts:
        BTreeMap<String, FirstDifferenceTransitionStats>,
    compact_numeric_ascii_suffixes: BTreeMap<String, PendingRuleReviewClusterStats>,
    grade1_shortform_prefix_surfaces: BTreeMap<String, PendingRuleReviewClusterStats>,
    grade1_numeric_continuation_surfaces: BTreeMap<String, PendingRuleReviewClusterStats>,
    grade1_hyphen_continuation_surfaces: BTreeMap<String, PendingRuleReviewClusterStats>,
    overlapping_traits: BTreeMap<String, usize>,
    shards: BTreeMap<String, ShardStats>,
    samples: BTreeMap<String, Vec<Sample>>,
}

#[derive(Debug)]
struct Config {
    report_path: PathBuf,
    json_path: PathBuf,
    sample_limit: usize,
    threads: usize,
}

impl Config {
    fn parse() -> Result<Self, String> {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut config = Self {
            report_path: workspace.join("docs/corpus-analysis/NIKL_2025_V1.md"),
            json_path: workspace.join("target/nikl-corpus-analysis.json"),
            sample_limit: 5,
            threads: thread::available_parallelism()
                .map_or(1, usize::from)
                .min(8),
        };

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--report" => {
                    config.report_path =
                        PathBuf::from(args.next().ok_or("--report requires a path")?);
                }
                "--json" => {
                    config.json_path = PathBuf::from(args.next().ok_or("--json requires a path")?);
                }
                "--sample-limit" => {
                    config.sample_limit = args
                        .next()
                        .ok_or("--sample-limit requires a number")?
                        .parse()
                        .map_err(|_| "--sample-limit must be a positive integer")?;
                }
                "--threads" => {
                    config.threads = args
                        .next()
                        .ok_or("--threads requires a number")?
                        .parse()
                        .map_err(|_| "--threads must be a positive integer")?;
                    if config.threads == 0 {
                        return Err("--threads must be at least 1".to_string());
                    }
                }
                "--help" | "-h" => {
                    println!(
                        "nikl_corpus_analyze [--report PATH] [--json PATH] \
                         [--sample-limit N] [--threads N]"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }
        Ok(config)
    }
}

fn load_cases() -> Result<Vec<LocatedCase>, String> {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_cases/corpus");
    let mut paths = fs::read_dir(&corpus_dir)
        .map_err(|error| format!("cannot read {}: {error}", corpus_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate corpus shards: {error}"))?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("sentence_") && name.ends_with(".json"))
    });
    paths.sort();
    let shard_count = paths.len();

    let mut located = Vec::new();
    for path in paths {
        let shard = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("non-Unicode shard path: {}", path.display()))?
            .to_string();
        let cases: Vec<CorpusCase> = serde_json::from_reader(BufReader::new(
            File::open(&path)
                .map_err(|error| format!("cannot open {}: {error}", path.display()))?,
        ))
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;
        located.extend(
            cases
                .into_iter()
                .enumerate()
                .map(|(index, case)| LocatedCase {
                    shard: shard.clone(),
                    index: index + 1,
                    case,
                }),
        );
    }
    validate_corpus_shape(shard_count, located.len())?;
    Ok(located)
}

fn validate_corpus_shape(shard_count: usize, case_count: usize) -> Result<(), String> {
    if shard_count == 0 {
        return Err("no NIKL corpus shards matched sentence_*.json".to_string());
    }
    if case_count == 0 {
        return Err("NIKL corpus shards contained zero cases".to_string());
    }
    Ok(())
}

fn singleton_unsupported_set_with(
    cases: &[LocatedCase],
    mut fails_alone: impl FnMut(char) -> bool,
) -> BTreeSet<char> {
    cases
        .iter()
        .flat_map(|located| located.case.input.chars())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|ch| fails_alone(*ch))
        .collect()
}

fn singleton_unsupported_set(cases: &[LocatedCase]) -> BTreeSet<char> {
    singleton_unsupported_set_with(cases, |ch| {
        braillify::encode_to_unicode(&ch.to_string()).is_err()
    })
}

fn encode_cases(cases: &[LocatedCase], thread_count: usize) -> Vec<EncodedCase> {
    // Compute singleton support exactly once per distinct corpus character.
    // `BTreeSet` keeps both probing and report assignment deterministic; workers
    // only perform membership lookups instead of re-encoding common marks such
    // as `㈜` hundreds of times across error sentences.
    let singleton_unsupported = singleton_unsupported_set(cases);
    let chunk_size = cases.len().div_ceil(thread_count);
    let mut chunks = thread::scope(|scope| {
        let singleton_unsupported = &singleton_unsupported;
        cases
            .chunks(chunk_size.max(1))
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .cloned()
                        .map(|located| {
                            let actual = braillify::encode_to_unicode(&located.case.input);
                            let singleton_unsupported_characters = if actual.is_err() {
                                located
                                    .case
                                    .input
                                    .chars()
                                    .collect::<BTreeSet<_>>()
                                    .into_iter()
                                    .filter(|ch| singleton_unsupported.contains(ch))
                                    .collect()
                            } else {
                                Vec::new()
                            };
                            let nfc: String = located.case.input.nfc().collect();
                            let nfc_actual = (nfc != located.case.input)
                                .then(|| braillify::encode_to_unicode(&nfc));
                            let nfkc: String = located.case.input.nfkc().collect();
                            let nfkc_actual = (nfkc != located.case.input)
                                .then(|| braillify::encode_to_unicode(&nfkc));
                            EncodedCase {
                                located,
                                actual,
                                nfc_actual,
                                nfkc_actual,
                                singleton_unsupported_characters,
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("analysis worker panicked"))
            .collect::<Vec<_>>()
    });
    chunks.drain(..).flatten().collect()
}

fn normalized_braille_whitespace(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            ' ' | '\t' | '\r' | '\n' | '\u{00a0}' | '\u{3000}' => '\u{2800}',
            _ => ch,
        })
        .collect()
}

/// Correct only the ordering defect justified by Korean rules 28 appendix and 29:
/// the Korean roman indicator must precede UEB grade-1/capital indicators.
fn roman_before_capital_order(text: &str) -> String {
    text.replace("⠠⠠⠠⠴", "⠴⠠⠠⠠")
        .replace("⠰⠠⠠⠴", "⠴⠰⠠⠠")
        .replace("⠠⠠⠴", "⠴⠠⠠")
}

fn conflicting_inputs(cases: &[LocatedCase]) -> (usize, BTreeSet<String>) {
    let mut references = BTreeMap::<String, BTreeSet<String>>::new();
    for located in cases {
        references
            .entry(located.case.input.clone())
            .or_default()
            .insert(located.case.unicode.clone());
    }
    let duplicate_count = cases.len().saturating_sub(references.len());
    let conflicting = references
        .into_iter()
        .filter_map(|(input, values)| (values.len() > 1).then_some(input))
        .collect();
    (duplicate_count, conflicting)
}

fn classify(encoded: &EncodedCase, conflicting: &BTreeSet<String>) -> (PrimaryClass, Reason) {
    let expected = &encoded.located.case.unicode;
    match &encoded.actual {
        Ok(actual) if actual == expected => (PrimaryClass::Exact, Reason::Exact),
        _ if conflicting.contains(&encoded.located.case.input) => (
            PrimaryClass::CorpusSuspect,
            Reason::ConflictingDuplicateReference,
        ),
        Ok(actual)
            if normalized_braille_whitespace(actual) == normalized_braille_whitespace(expected) =>
        {
            (
                PrimaryClass::ComparisonMethod,
                Reason::BrailleWhitespaceEquivalent,
            )
        }
        _ if encoded
            .nfc_actual
            .as_ref()
            .is_some_and(|result| result.as_ref().is_ok_and(|actual| actual == expected)) =>
        {
            (PrimaryClass::ComparisonMethod, Reason::NfcInputEquivalent)
        }
        _ if encoded
            .nfkc_actual
            .as_ref()
            .is_some_and(|result| result.as_ref().is_ok_and(|actual| actual == expected)) =>
        {
            (PrimaryClass::ComparisonMethod, Reason::NfkcInputEquivalent)
        }
        _ if is_rule_34_reference_order_contradiction(encoded) => (
            PrimaryClass::CorpusSuspect,
            Reason::Rule34RomanIndicatorBeforeOpeningParenthesis,
        ),
        Ok(actual) if roman_before_capital_order(actual) == *expected => (
            PrimaryClass::ImplementationDefect,
            Reason::RomanIndicatorAfterCapitalIndicator,
        ),
        Err(_) if !encoded.singleton_unsupported_characters.is_empty() => (
            PrimaryClass::UnsupportedCharacterReview,
            Reason::UnsupportedCharacterReview,
        ),
        Err(_) => (
            PrimaryClass::UnclassifiedEncodingErrorReview,
            Reason::UnclassifiedEncodingErrorReview,
        ),
        Ok(_)
            if encoded
                .located
                .case
                .input
                .chars()
                .any(|ch| ch.is_ascii_alphabetic()) =>
        {
            (
                PrimaryClass::PendingRuleReview,
                Reason::ForeignTextRuleReview,
            )
        }
        Ok(_)
            if encoded
                .located
                .case
                .input
                .chars()
                .any(|ch| ch.is_ascii_digit()) =>
        {
            (PrimaryClass::PendingRuleReview, Reason::NumberRuleReview)
        }
        Ok(_)
            if encoded
                .located
                .case
                .input
                .chars()
                .any(is_delimiter_or_quote) =>
        {
            (
                PrimaryClass::PendingRuleReview,
                Reason::PunctuationRuleReview,
            )
        }
        Ok(_) => (PrimaryClass::PendingRuleReview, Reason::KoreanRuleReview),
    }
}

fn is_roman_numeral_presentation(ch: char) -> bool {
    (0x2160..=0x217f).contains(&(ch as u32))
}

/// Reconstruct an observable before/after transition for the rule-36 cohort
/// without assigning today's primary-class policy to the historical run.
/// Before targeted normalization, direct/NFC encoding failed; an exact NFKC
/// path was observable separately. The current side reports only whether the
/// case is exact, an encoded mismatch awaiting rule review, or still blocked by
/// another independently unsupported singleton character.
fn rule_36_observed_transition(encoded: &EncodedCase) -> Option<&'static str> {
    if !encoded
        .located
        .case
        .input
        .chars()
        .any(is_roman_numeral_presentation)
    {
        return None;
    }

    let expected = &encoded.located.case.unicode;
    let before = if encoded
        .nfkc_actual
        .as_ref()
        .is_some_and(|result| result.as_ref().is_ok_and(|actual| actual == expected))
    {
        "nfkc_input_equivalent"
    } else {
        "encoding_error"
    };
    let after = match &encoded.actual {
        Ok(actual) if actual == expected => "exact",
        Ok(_) => "encoded_mismatch_pending_rule_review",
        Err(_) if !encoded.singleton_unsupported_characters.is_empty() => {
            "unsupported_character_review"
        }
        Err(_) => "unclassified_encoding_error_review",
    };
    Some(match (before, after) {
        ("nfkc_input_equivalent", "exact") => "nfkc_input_equivalent -> exact",
        ("encoding_error", "encoded_mismatch_pending_rule_review") => {
            "encoding_error -> encoded_mismatch_pending_rule_review"
        }
        ("encoding_error", "unsupported_character_review") => {
            "encoding_error -> unsupported_character_review"
        }
        ("encoding_error", "unclassified_encoding_error_review") => {
            "encoding_error -> unclassified_encoding_error_review"
        }
        _ => "other_observed_transition",
    })
}

fn is_delimiter_or_quote(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')' | '[' | ']' | '{' | '}' | '“' | '”' | '‘' | '’' | '"' | '\''
    )
}

const UPPERCASE_ROMAN_HEADWORD_EXPANSION: &str =
    "uppercase_roman_headword_closed_multiword_parenthetical";
const STANDALONE_UPPERCASE_ROMAN_WORD: &str = "standalone_multi_character_uppercase_roman_word";
const KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL: &str = "korean_prefixed_closed_allcaps_parenthetical";
const KOREAN_PREFIXED_CLOSED_ROMAN_ANNOTATION: &str =
    "korean_prefixed_closed_roman_annotation_rule_34_order";
const ALLCAPS_ROMAN_MIDDLE_DOT_RUNS: &str =
    "multi_character_allcaps_roman_runs_joined_by_middle_dot";
const ROMAN_RUN_BEFORE_MIDDLE_DOT_BOUNDARY: &str =
    "roman_run_immediately_before_attached_middle_dot_boundary";
const KOREAN_INLINE_PARENTHESIZED_OPERATOR: &str =
    "korean_inline_parenthesized_single_arithmetic_operator";
const TIGHT_TRIANGLE_BEFORE_KOREAN: &str = "tight_triangle_mark_immediately_before_korean";
const ALLCAPS_ROMAN_RUN_CONTAINING_OU: &str = "allcaps_roman_run_containing_ou";
const SINGLE_CAPITAL_PARENTHESIZED_DIGITS: &str = "single_capital_followed_by_parenthesized_digits";
const MIXED_ROMAN_KOREAN_BEFORE_HEADWORD_EXPANSION: &str =
    "mixed_roman_korean_word_before_uppercase_headword_expansion";
const UPPERCASE_ROMAN_HYPHEN_DIGITS: &str = "uppercase_roman_run_followed_by_hyphen_digits";
const DECIMAL_POINT_BETWEEN_DIGITS: &str = "decimal_point_between_ascii_digits";
const COMPACT_NUMERIC_ASCII_SUFFIX: &str = "compact_numeric_ascii_letter_suffix";
const RULE69_ASCII_UNIT_BEFORE_TERMINATOR_SKIPPING_SYMBOL: &str =
    "rule69_ascii_unit_before_terminator_skipping_symbol";
const ALLCAPS_SHORTFORM_PREFIX_COLLISION: &str =
    "allcaps_roman_run_beginning_with_pure_letter_shortform";
const ROMAN_UPPERCASE_AFTER_DIGIT: &str =
    "uppercase_ascii_run_immediately_after_digit_in_roman_sequence";
const ROMAN_UPPERCASE_AFTER_HYPHEN: &str =
    "uppercase_ascii_run_immediately_after_hyphen_in_roman_sequence";
const PURE_ALLCAPS_HYPHEN_MULTI_ALLCAPS: &str =
    "pure_allcaps_segment_before_hyphen_and_multi_allcaps_segment_after";
const ROMAN_HYPHENATED_WORD_AFTER_KOREAN_WORD: &str =
    "roman_hyphenated_word_after_whitespace_following_korean_word";
const ROMAN_PARENTHETICAL_HEADWORD_AFTER_KOREAN_WORD: &str =
    "roman_parenthetical_headword_after_whitespace_following_korean_word";
const KOREAN_PREFIXED_ROMAN_PARENTHETICAL_HYPHEN_SUFFIX: &str =
    "korean_prefixed_roman_parenthetical_followed_by_allcaps_hyphen_suffix";
const CONSECUTIVE_ROMAN_UPPERCASE_WORD_REENTRY: &str =
    "uppercase_word_after_whitespace_continuing_ascii_roman_text";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InputSpan {
    start_byte: usize,
    end_byte: usize,
}

/// Finds whitespace-delimited words containing an ASCII decimal point between
/// digits. Rule 43 explicitly keeps punctuation between digits in the same
/// numeric sequence, and rule 48 assigns the decimal-point cell. The whole
/// word is retained so the output locator reproduces suffix contexts such as
/// `%`, Roman units, Korean text, and closing punctuation.
fn decimal_word_spans(input: &str) -> Vec<InputSpan> {
    let mut spans = BTreeSet::new();
    for (dot_byte, _) in input.match_indices('.') {
        let previous = input[..dot_byte].chars().next_back();
        let next = input[dot_byte + 1..].chars().next();
        if !previous.is_some_and(|ch| ch.is_ascii_digit())
            || !next.is_some_and(|ch| ch.is_ascii_digit())
        {
            continue;
        }

        let start_byte = input[..dot_byte]
            .char_indices()
            .rev()
            .find_map(|(byte, ch)| ch.is_whitespace().then_some(byte + ch.len_utf8()))
            .unwrap_or(0);
        let end_byte = input[dot_byte + 1..]
            .char_indices()
            .find_map(|(offset, ch)| ch.is_whitespace().then_some(dot_byte + 1 + offset))
            .unwrap_or(input.len());
        spans.insert((start_byte, end_byte));
    }
    spans
        .into_iter()
        .map(|(start_byte, end_byte)| InputSpan {
            start_byte,
            end_byte,
        })
        .collect()
}

/// Finds a compact numeric prefix followed immediately by one or more ASCII
/// letters, with alphanumeric outer boundaries. The shape includes rule-69
/// units but deliberately does not declare every suffix a unit: mathematical
/// variables and identifiers can share the same surface form.
fn compact_numeric_ascii_suffix_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_digit()
            || input[..cursor]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric())
        {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }

        let start_byte = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(*byte, b',' | b'.'))
        {
            cursor += 1;
        }
        let suffix_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_alphabetic) {
            cursor += 1;
        }
        if cursor > suffix_start
            && input[cursor..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric())
        {
            spans.push(InputSpan {
                start_byte,
                end_byte: cursor,
            });
        }
    }
    spans
}

fn compact_numeric_ascii_suffix(span: InputSpan, input: &str) -> &str {
    input[span.start_byte..span.end_byte]
        .trim_start_matches(|ch: char| ch.is_ascii_digit() || matches!(ch, ',' | '.'))
}

/// Refines the broad compact-suffix cohort to spellings already recognized by
/// rule 69, immediately followed by punctuation for which rules 33/34 omit the
/// Roman terminator. The span includes that punctuation so the output locator
/// measures the exact unit-to-punctuation boundary rather than mere coexistence.
fn rule69_ascii_unit_before_terminator_skipping_symbol_spans(input: &str) -> Vec<InputSpan> {
    const RULE69_ASCII_UNITS: &[&str] = &["min", "cal", "cm", "kg", "in", "mm", "GB", "m", "h"];

    compact_numeric_ascii_suffix_spans(input)
        .into_iter()
        .filter_map(|span| {
            let suffix = compact_numeric_ascii_suffix(span, input);
            let symbol = input[span.end_byte..].chars().next()?;
            (RULE69_ASCII_UNITS.contains(&suffix)
                && matches!(
                    symbol,
                    '.' | '?'
                        | '!'
                        | '…'
                        | '⋯'
                        | '"'
                        | '\''
                        | '”'
                        | '’'
                        | '」'
                        | '』'
                        | '〉'
                        | '》'
                        | '('
                        | ')'
                        | ']'
                        | '}'
                        | ','
                        | ':'
                        | ';'
                        | '―'
                ))
            .then_some(InputSpan {
                start_byte: span.start_byte,
                end_byte: span.end_byte + symbol.len_utf8(),
            })
        })
        .collect()
}

fn first_difference_at_rule69_ascii_unit_terminator_boundary(item: &EncodedCase) -> bool {
    first_difference_in_compact_numeric_ascii_suffix_spans(
        item,
        &rule69_ascii_unit_before_terminator_skipping_symbol_spans(&item.located.case.input),
    )
}

fn first_difference_in_compact_numeric_ascii_suffix(item: &EncodedCase) -> bool {
    first_difference_in_compact_numeric_ascii_suffix_spans(
        item,
        &compact_numeric_ascii_suffix_spans(&item.located.case.input),
    )
}

fn first_difference_in_compact_numeric_ascii_suffix_spans(
    item: &EncodedCase,
    spans: &[InputSpan],
) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    korean_context_signature_ranges(&item.located.case.input, actual, spans, 1)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

fn first_difference_in_decimal_word(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    korean_context_signature_ranges(
        &item.located.case.input,
        actual,
        &decimal_word_spans(&item.located.case.input),
        0,
    )
    .into_iter()
    .any(|range| range.contains(&first_difference))
}

/// Finds rule-34-shaped annotations whose opening parenthesis immediately
/// follows Korean script and whose closed body contains only ordinary Roman
/// letters, digits, apostrophes, periods, or hyphens. This is an input gate;
/// the separate output locator decides whether a first difference is at the
/// opening-parenthesis order established by the PDF example.
fn korean_prefixed_closed_roman_annotation_spans(input: &str) -> Vec<InputSpan> {
    let mut spans = Vec::new();
    for (open_byte, _) in input.match_indices('(') {
        if !input[..open_byte]
            .chars()
            .next_back()
            .is_some_and(is_korean_script)
        {
            continue;
        }

        let body_start = open_byte + 1;
        let Some(close_offset) = input[body_start..].find(')') else {
            continue;
        };
        let close_byte = body_start + close_offset;
        let body = &input[body_start..close_byte];
        if !body.is_empty()
            && body.chars().any(|ch| ch.is_ascii_alphabetic())
            && body
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '\'' | '.'))
        {
            spans.push(InputSpan {
                start_byte: open_byte,
                end_byte: close_byte + 1,
            });
        }
    }
    spans
}

/// Locate only the current engine's Korean opening-parenthesis cells. The
/// signature is derived from a neutral Korean probe and verified at the cell
/// offset obtained by encoding the real prefix, never from corpus expected.
fn korean_prefixed_annotation_opening_ranges(
    input: &str,
    actual: &str,
) -> Vec<std::ops::Range<usize>> {
    let actual_cells = actual.chars().collect::<Vec<_>>();
    let korean = braillify::encode_to_unicode("가").expect("neutral Korean probe must encode");
    let korean_with_open =
        braillify::encode_to_unicode("가(").expect("Korean opening-parenthesis probe must encode");
    let korean_cells = korean.chars().count();
    let opening = korean_with_open
        .chars()
        .skip(korean_cells)
        .collect::<Vec<_>>();

    korean_prefixed_closed_roman_annotation_spans(input)
        .into_iter()
        .filter_map(|span| {
            let prefix = braillify::encode_to_unicode(&input[..span.start_byte]).ok()?;
            let start = prefix.chars().count();
            let end = start.checked_add(opening.len())?;
            (actual_cells.get(start..end) == Some(opening.as_slice())).then_some(start..end)
        })
        .collect()
}

fn first_difference_in_korean_prefixed_annotation_opening(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    korean_prefixed_annotation_opening_ranges(&item.located.case.input, actual)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

/// The PDF's rule-34 example emits the printed Korean opening parenthesis
/// before entering Roman mode: `⠦⠄⠴`. A corpus reference that instead starts
/// this same localized input structure with Roman mode plus the UEB opening
/// parenthesis (`⠴⠐⠣`) contradicts that explicit order. Requiring both
/// three-cell signatures avoids reclassifying unrelated mismatches in the
/// broad input cohort.
fn is_rule_34_reference_order_contradiction(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    let expected = &item.located.case.unicode;
    if actual == expected {
        return false;
    }

    let difference = first_difference_cell(expected, actual);
    let expected_cells = expected.chars().collect::<Vec<_>>();
    let actual_cells = actual.chars().collect::<Vec<_>>();
    expected_cells.get(difference..difference + 3) == Some(&['⠴', '⠐', '⠣'])
        && actual_cells.get(difference..difference + 3) == Some(&['⠦', '⠄', '⠴'])
        && korean_prefixed_annotation_opening_ranges(&item.located.case.input, actual)
            .into_iter()
            .any(|range| range.start == difference)
}

/// Finds maximal all-caps ASCII runs containing the adjacent letters `OU`.
///
/// This is an input gate for a pronunciation-sensitive UEB diagnostic, not a
/// claim that the run is an initialism. Alphanumeric outer boundaries exclude
/// fragments of identifiers while retaining parenthesized and standalone runs.
fn allcaps_roman_runs_containing_ou(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut runs = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphabetic() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }

        let start_byte = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_alphabetic() {
            cursor += 1;
        }
        let end_byte = cursor;
        let run = &input[start_byte..end_byte];
        let previous = input[..start_byte].chars().next_back();
        let next = input[end_byte..].chars().next();
        if run.len() >= 2
            && run.bytes().all(|byte| byte.is_ascii_uppercase())
            && run.as_bytes().windows(2).any(|pair| pair == b"OU")
            && previous.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && next.is_none_or(|ch| !ch.is_ascii_alphanumeric())
        {
            runs.push(InputSpan {
                start_byte,
                end_byte,
            });
        }
    }
    runs
}

/// Finds maximal pure-uppercase ASCII letter runs whose beginning is itself a
/// pure-letter UEB shortform abbreviation. UEB 5.7.2 and 10.9.7-10.9.8 require
/// grade 1 both for a complete shortform-shaped letters-sequence (`WD`) and for
/// a longer word beginning with one (`PDS`, `LLM`, `GDP`).
fn allcaps_shortform_prefix_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphabetic() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }

        let start_byte = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_alphabetic) {
            cursor += 1;
        }
        let end_byte = cursor;
        let run = &input[start_byte..end_byte];
        // The isolated prefix is current-engine evidence only: no corpus
        // expected/reference value participates in this candidate gate. For a
        // pure all-caps letters-sequence, a leading ⠰ is the engine's existing
        // UEB 5.7.2/10.9.7 shortform-collision decision.
        let has_shortform_prefix = (2..=run.len()).any(|end| {
            braillify::encode_to_unicode(&run[..end]).is_ok_and(|encoded| encoded.starts_with('⠰'))
        });
        if run.len() >= 2
            && run.bytes().all(|byte| byte.is_ascii_uppercase())
            && has_shortform_prefix
            && input[..start_byte]
                .chars()
                .next_back()
                .is_none_or(|previous| !previous.is_ascii_alphanumeric())
            && input[end_byte..]
                .chars()
                .next()
                .is_none_or(|next| !next.is_ascii_alphanumeric())
        {
            spans.push(InputSpan {
                start_byte,
                end_byte,
            });
        }
    }
    spans
}

/// Finds maximal ASCII alphanumeric identifiers containing an immediate
/// digit-to-uppercase transition (`O4O`, `Li2S`, `V2X`). The numeric indicator
/// itself sets grade-1 mode under UEB 5.6.1, and 5.6.2 does not terminate that
/// mode at a capital indicator; the cohort measures whether an extra `⠰` is
/// nevertheless emitted at this exact boundary.
fn roman_uppercase_after_digit_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphanumeric() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }
        let start_byte = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_alphanumeric) {
            cursor += 1;
        }
        let end_byte = cursor;
        let run = &bytes[start_byte..end_byte];
        if run.iter().any(u8::is_ascii_alphabetic)
            && run
                .windows(2)
                .any(|pair| pair[0].is_ascii_digit() && pair[1].is_ascii_uppercase())
        {
            spans.push(InputSpan {
                start_byte,
                end_byte,
            });
        }
    }
    spans
}

/// Finds maximal ASCII Roman identifiers containing a hyphen immediately
/// followed by an uppercase run (`U-ENTER`, `CD-ROM`). This is independent of
/// the digit transition above: Korean rule 29 keeps one Roman section around
/// consecutive Roman text, while UEB 5.6.2 gives a hyphen separate significance
/// only when terminating numeric grade-1 mode.
fn roman_uppercase_after_hyphen_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphanumeric() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }
        let start_byte = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            cursor += 1;
        }
        let end_byte = cursor;
        let run = &bytes[start_byte..end_byte];
        if run.iter().any(u8::is_ascii_alphabetic)
            && run
                .windows(2)
                .any(|pair| pair[0] == b'-' && pair[1].is_ascii_uppercase())
        {
            spans.push(InputSpan {
                start_byte,
                end_byte,
            });
        }
    }
    spans
}

/// Narrows the broad hyphen continuation diagnostic to the UEB 5.7.2
/// `CD-ROM` boundary implemented by the engine: the immediately adjacent
/// letter segment before the hyphen is pure uppercase, and the immediately
/// adjacent segment after it is pure uppercase with at least two letters.
fn pure_allcaps_hyphen_multi_allcaps_spans(input: &str) -> Vec<InputSpan> {
    roman_uppercase_after_hyphen_spans(input)
        .into_iter()
        .filter(|span| {
            let run = &input.as_bytes()[span.start_byte..span.end_byte];
            run.iter().enumerate().any(|(hyphen, byte)| {
                if *byte != b'-' {
                    return false;
                }
                let prefix_start = run[..hyphen]
                    .iter()
                    .rposition(|byte| !byte.is_ascii_alphabetic())
                    .map_or(0, |index| index + 1);
                let prefix = &run[prefix_start..hyphen];
                let suffix = &run[hyphen + 1..];
                let suffix_len = suffix
                    .iter()
                    .take_while(|byte| byte.is_ascii_alphabetic())
                    .count();
                let suffix_letters = &suffix[..suffix_len];

                !prefix.is_empty()
                    && prefix.iter().all(u8::is_ascii_uppercase)
                    && suffix_letters.len() >= 2
                    && suffix_letters.iter().all(u8::is_ascii_uppercase)
            })
        })
        .collect()
}

fn preceding_whitespace_word_contains_korean(input: &str, start_byte: usize) -> bool {
    let before = &input[..start_byte];
    if !before.chars().next_back().is_some_and(char::is_whitespace) {
        return false;
    }
    before
        .trim_end_matches(char::is_whitespace)
        .rsplit(char::is_whitespace)
        .next()
        .is_some_and(|word| word.chars().any(is_korean_script))
}

/// Narrows the broad hyphen-continuation trait to a new Roman word after a
/// whitespace-delimited Korean-containing word. This separates entry-mode
/// routing (`A-STAR`) from the already measured grade-1 boundary inside a
/// Roman run (`CD-ROM`). It remains diagnostic because math rule 2 gives the
/// same hyphen-minus a subtraction reading.
fn roman_hyphenated_word_after_korean_word_spans(input: &str) -> Vec<InputSpan> {
    roman_uppercase_after_hyphen_spans(input)
        .into_iter()
        .filter(|span| {
            input.as_bytes()[span.start_byte].is_ascii_alphabetic()
                && preceding_whitespace_word_contains_korean(input, span.start_byte)
        })
        .collect()
}

/// Finds a two-or-more-letter Roman headword immediately after a whitespace-
/// delimited Korean-containing word and immediately before a non-empty closed
/// parenthetical. The parenthetical may contain prose or notation; no semantic
/// choice between Korean rule 29 and math rules 6/12/45 is inferred.
fn roman_parenthetical_headword_after_korean_word_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (open, _) in input.match_indices('(') {
        let mut start_byte = open;
        while start_byte > 0 && bytes[start_byte - 1].is_ascii_alphabetic() {
            start_byte -= 1;
        }
        if open - start_byte < 2 || !preceding_whitespace_word_contains_korean(input, start_byte) {
            continue;
        }
        let body_start = open + 1;
        let Some((close_offset, close)) = input[body_start..]
            .char_indices()
            .find(|(_, ch)| matches!(ch, '(' | ')'))
        else {
            continue;
        };
        if close == ')' && close_offset > 0 {
            spans.push(InputSpan {
                start_byte,
                end_byte: open,
            });
        }
    }
    spans
}

/// Separates an attached rule-34-shaped Roman parenthetical followed by a
/// hyphenated all-caps suffix from both whitespace Roman entry and ordinary
/// all-caps hyphen continuation. The strict body/suffix gate is structural
/// evidence only; rule 34 and math rules 6/12 still permit competing modes.
fn korean_prefixed_roman_parenthetical_hyphen_suffix_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (open, _) in input.match_indices('(') {
        if !input[..open]
            .chars()
            .next_back()
            .is_some_and(is_korean_script)
        {
            continue;
        }
        let body_start = open + 1;
        let Some(close_offset) = input[body_start..].find(')') else {
            continue;
        };
        let close = body_start + close_offset;
        let body = &input[body_start..close];
        if body.len() < 2 || !body.bytes().all(|byte| byte.is_ascii_uppercase()) {
            continue;
        }
        if bytes.get(close + 1) != Some(&b'-') {
            continue;
        }
        let suffix_start = close + 2;
        let mut end_byte = suffix_start;
        while bytes.get(end_byte).is_some_and(u8::is_ascii_uppercase) {
            end_byte += 1;
        }
        if end_byte - suffix_start < 2
            || input[end_byte..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphanumeric())
        {
            continue;
        }
        spans.push(InputSpan {
            start_byte: open,
            end_byte,
        });
    }
    spans
}

/// Finds an uppercase word-start run after whitespace when the preceding
/// whitespace-delimited word ends in an ASCII letter. Korean rule 29 treats
/// consecutive Roman text as one section, but the uppercase token phase can
/// independently request another entry before this second word. Punctuation
/// and non-letter endings deliberately break this structural gate.
fn consecutive_roman_uppercase_word_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (start_byte, ch) in input.char_indices() {
        if !ch.is_ascii_uppercase()
            || !input[..start_byte]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
        {
            continue;
        }
        let before = input[..start_byte].trim_end_matches(char::is_whitespace);
        if !before
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            continue;
        }
        let mut end_byte = start_byte;
        while bytes.get(end_byte).is_some_and(u8::is_ascii_uppercase) {
            end_byte += 1;
        }
        if end_byte - start_byte >= 2
            && !input[end_byte..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphabetic())
        {
            spans.push(InputSpan {
                start_byte,
                end_byte,
            });
        }
    }
    spans
}

/// Locates each detected run in the full current-engine output by searching
/// for that run's independently encoded signature. This uses neither the
/// corpus reference nor a hard-coded braille value.
fn current_engine_signature_ranges(
    input: &str,
    actual: &str,
    spans: &[InputSpan],
    leading_boundary_cells: usize,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = BTreeSet::new();
    for candidate in spans {
        let run = &input[candidate.start_byte..candidate.end_byte];
        let Ok(signature) = braillify::encode_to_unicode(run) else {
            continue;
        };
        let signature_cells = signature.chars().count();
        for (start_byte, _) in actual.match_indices(&signature) {
            let signature_start = actual[..start_byte].chars().count();
            let start = signature_start.saturating_sub(leading_boundary_cells);
            ranges.insert((start, signature_start + signature_cells));
        }
    }
    ranges.into_iter().map(|(start, end)| start..end).collect()
}

/// Produces the current mixed-Korean routing signature for a candidate. This
/// differs from encoding the candidate in isolation when the pure-English UEB
/// preflight owns the isolated text but the mixed document routes it as math.
fn korean_context_signature(run: &str) -> Option<String> {
    let left = braillify::encode_to_unicode("가").ok()?;
    let right = braillify::encode_to_unicode("나").ok()?;
    let probe = braillify::encode_to_unicode(&format!("가 {run} 나")).ok()?;
    let probe_cells = probe.chars().collect::<Vec<_>>();
    let start = left.chars().count();
    let end = probe_cells.len().checked_sub(right.chars().count())?;
    let middle = probe_cells.get(start..end)?;
    let first_content = middle.iter().position(|cell| *cell != '⠀')?;
    let last_content = middle.iter().rposition(|cell| *cell != '⠀')?;
    Some(middle[first_content..=last_content].iter().collect())
}

fn korean_context_signature_ranges(
    input: &str,
    actual: &str,
    spans: &[InputSpan],
    leading_boundary_cells: usize,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = BTreeSet::new();
    for candidate in spans {
        let run = &input[candidate.start_byte..candidate.end_byte];
        let Some(signature) = korean_context_signature(run) else {
            continue;
        };
        let signature_cells = signature.chars().count();
        for (start_byte, _) in actual.match_indices(&signature) {
            let signature_start = actual[..start_byte].chars().count();
            ranges.insert((
                signature_start.saturating_sub(leading_boundary_cells),
                signature_start + signature_cells,
            ));
        }
    }
    ranges.into_iter().map(|(start, end)| start..end).collect()
}

/// Produces the current signature of a Roman candidate embedded inside one
/// mixed Korean word. This intentionally complements the space-delimited probe
/// above: token-level capitalization can add grade 1 to standalone `WD`, while
/// the residual under review occurs in attached forms such as `한글(WD)`.
fn mixed_korean_word_signature(run: &str) -> Option<String> {
    let left = braillify::encode_to_unicode("가").ok()?;
    let right = braillify::encode_to_unicode("나").ok()?;
    let probe = braillify::encode_to_unicode(&format!("가{run}나")).ok()?;
    let probe_cells = probe.chars().collect::<Vec<_>>();
    let start = left.chars().count();
    let end = probe_cells.len().checked_sub(right.chars().count())?;
    Some(probe_cells.get(start..end)?.iter().collect())
}

fn roman_entry_signature_ranges(
    input: &str,
    actual: &str,
    spans: &[InputSpan],
    leading_boundary_cells: usize,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = BTreeSet::new();
    for candidate in spans {
        let run = &input[candidate.start_byte..candidate.end_byte];
        for signature in [
            korean_context_signature(run),
            mixed_korean_word_signature(run),
        ]
        .into_iter()
        .flatten()
        {
            // The neutral Korean probe appends its own current-engine exit cell.
            // A corpus candidate followed by a closing parenthesis can suppress
            // that exit under Korean rule 34, so search both the complete probe
            // and the same current-engine signature without only that generated
            // trailing boundary. Candidate letters and indicators are untouched.
            let without_probe_exit = signature
                .char_indices()
                .next_back()
                .map(|(last, _)| signature[..last].to_string());
            for searchable in [Some(signature), without_probe_exit]
                .into_iter()
                .flatten()
                .filter(|candidate| !candidate.is_empty())
            {
                let signature_cells = searchable.chars().count();
                for (start_byte, _) in actual.match_indices(&searchable) {
                    let signature_start = actual[..start_byte].chars().count();
                    ranges.insert((
                        signature_start.saturating_sub(leading_boundary_cells),
                        signature_start + signature_cells,
                    ));
                }
            }
        }
    }
    ranges.into_iter().map(|(start, end)| start..end).collect()
}

fn first_difference_in_grade1_cohort_spans(item: &EncodedCase, spans: &[InputSpan]) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    let expected_cell = item.located.case.unicode.chars().nth(first_difference);
    let actual_cell = actual.chars().nth(first_difference);
    if !matches!(
        (expected_cell, actual_cell),
        (Some('\u{2830}'), Some('\u{2820}')) | (Some('\u{2820}'), Some('\u{2830}'))
    ) {
        return false;
    }
    roman_entry_signature_ranges(&item.located.case.input, actual, spans, 1)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

/// Locates the actual output boundary corresponding to an input span start.
/// The prefix is encoded independently and must be byte-for-byte equal to the
/// full output prefix, so a repeated Roman surface elsewhere cannot satisfy
/// the locator. The short range covers only mode-entry cells, not the run.
fn current_engine_input_entry_ranges(
    input: &str,
    actual: &str,
    spans: &[InputSpan],
    entry_cells: usize,
) -> Vec<std::ops::Range<usize>> {
    let actual_cells = actual.chars().collect::<Vec<_>>();
    let mut ranges = BTreeSet::new();
    for span in spans {
        let Ok(prefix) = braillify::encode_to_unicode(&input[..span.start_byte]) else {
            continue;
        };
        let prefix_cells = prefix.chars().collect::<Vec<_>>();
        if actual_cells.starts_with(&prefix_cells) {
            let start = prefix_cells.len();
            let end = start.saturating_add(entry_cells).min(actual_cells.len());
            if start < end {
                ranges.insert((start, end));
            }
        }
    }
    ranges.into_iter().map(|(start, end)| start..end).collect()
}

fn first_difference_at_input_span_entry(
    item: &EncodedCase,
    spans: &[InputSpan],
    entry_cells: usize,
) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    current_engine_input_entry_ranges(&item.located.case.input, actual, spans, entry_cells)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

fn allcaps_ou_actual_ranges(input: &str, actual: &str) -> Vec<std::ops::Range<usize>> {
    current_engine_signature_ranges(input, actual, &allcaps_roman_runs_containing_ou(input), 0)
}

fn first_difference_in_allcaps_ou_run(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    allcaps_ou_actual_ranges(&item.located.case.input, actual)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

/// Finds a standalone single capital immediately followed by a non-empty,
/// closed ASCII-digit parenthetical, such as `A(14)`. The span is deliberately
/// semantic-neutral: prose labels and mathematical function notation can share
/// this surface form.
fn single_capital_parenthesized_digit_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (start_byte, ch) in input.char_indices() {
        if !ch.is_ascii_uppercase()
            || input[..start_byte]
                .chars()
                .next_back()
                .is_some_and(|previous| previous.is_ascii_alphanumeric())
        {
            continue;
        }
        let open = start_byte + 1;
        if bytes.get(open) != Some(&b'(') {
            continue;
        }
        let mut cursor = open + 1;
        let digit_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == digit_start || bytes.get(cursor) != Some(&b')') {
            continue;
        }
        let end_byte = cursor + 1;
        if input[end_byte..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphanumeric())
        {
            continue;
        }
        spans.push(InputSpan {
            start_byte,
            end_byte,
        });
    }
    spans
}

/// Finds a maximal uppercase ASCII run followed by ASCII hyphen-minus and a
/// non-empty digit run, such as `D-100`, `F-35`, or `AH-64`. Identifier and
/// subtraction readings deliberately remain separate semantic possibilities.
fn uppercase_roman_hyphen_digit_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_uppercase() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }
        let start_byte = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_uppercase) {
            cursor += 1;
        }
        if input[..start_byte]
            .chars()
            .next_back()
            .is_some_and(|previous| previous.is_ascii_alphanumeric())
            || bytes.get(cursor) != Some(&b'-')
        {
            continue;
        }
        cursor += 1;
        let digit_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == digit_start
            || input[cursor..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphanumeric())
        {
            continue;
        }
        spans.push(InputSpan {
            start_byte,
            end_byte: cursor,
        });
    }
    spans
}

fn first_difference_in_signature_spans(
    item: &EncodedCase,
    spans: &[InputSpan],
    leading_boundary_cells: usize,
) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    current_engine_signature_ranges(
        &item.located.case.input,
        actual,
        spans,
        leading_boundary_cells,
    )
    .into_iter()
    .any(|range| range.contains(&first_difference))
}

fn first_difference_in_korean_context_signature_spans(
    item: &EncodedCase,
    spans: &[InputSpan],
    leading_boundary_cells: usize,
) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    korean_context_signature_ranges(
        &item.located.case.input,
        actual,
        spans,
        leading_boundary_cells,
    )
    .into_iter()
    .any(|range| range.contains(&first_difference))
}

/// Only output-localized cohorts may claim a first difference. Broad input-only
/// coexistence traits are intentionally absent: excluding them would hide
/// unrelated causes merely because a sentence also contains Roman text.
fn first_difference_claimed_by_prior_localized_cohort(item: &EncodedCase) -> bool {
    first_difference_in_allcaps_ou_run(item)
        || first_difference_in_compact_numeric_ascii_suffix(item)
        || first_difference_in_decimal_word(item)
        || first_difference_in_korean_prefixed_annotation_opening(item)
        || first_difference_in_inline_parenthesized_operator(item)
        || first_difference_in_tight_triangle(item)
        || first_difference_at_roman_middle_dot_boundary(item)
        || first_difference_in_signature_spans(
            item,
            &single_capital_parenthesized_digit_spans(&item.located.case.input),
            1,
        )
        || first_difference_in_signature_spans(
            item,
            &mixed_roman_korean_before_headword_expansion_spans(&item.located.case.input),
            1,
        )
        || first_difference_in_korean_context_signature_spans(
            item,
            &uppercase_roman_hyphen_digit_spans(&item.located.case.input),
            1,
        )
}

fn first_difference_claimed_before_roman_entry_residual(item: &EncodedCase) -> bool {
    first_difference_claimed_by_prior_localized_cohort(item)
        || first_difference_in_grade1_cohort_spans(
            item,
            &allcaps_shortform_prefix_spans(&item.located.case.input),
        )
        || first_difference_in_grade1_cohort_spans(
            item,
            &roman_uppercase_after_digit_spans(&item.located.case.input),
        )
        || first_difference_in_grade1_cohort_spans(
            item,
            &roman_uppercase_after_hyphen_spans(&item.located.case.input),
        )
}

fn first_difference_in_current_roman_entry_signature(
    item: &EncodedCase,
    spans: &[InputSpan],
) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    roman_entry_signature_ranges(&item.located.case.input, actual, spans, 0)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

fn first_difference_claimed_before_consecutive_roman_reentry(item: &EncodedCase) -> bool {
    first_difference_claimed_before_roman_entry_residual(item)
        || first_difference_at_input_span_entry(
            item,
            &roman_hyphenated_word_after_korean_word_spans(&item.located.case.input),
            2,
        )
        || first_difference_at_input_span_entry(
            item,
            &roman_parenthetical_headword_after_korean_word_spans(&item.located.case.input),
            2,
        )
        || first_difference_at_input_span_entry(
            item,
            &korean_prefixed_roman_parenthetical_hyphen_suffix_spans(&item.located.case.input),
            2,
        )
}

fn first_difference_claimed_by_localized_cohort(item: &EncodedCase) -> bool {
    first_difference_claimed_before_consecutive_roman_reentry(item)
        || first_difference_in_current_roman_entry_signature(
            item,
            &consecutive_roman_uppercase_word_spans(&item.located.case.input),
        )
}

/// Input-only candidate gate for acronym expansions such as
/// `HCA(Home Connectivity Alliance)`.
///
/// This is deliberately an analyzer diagnostic, not an engine rule. Requiring
/// only ASCII letters and spaces inside the closed parenthesis also excludes
/// visible operators, subscript/superscript notation, and nested parentheses.
fn uppercase_roman_headword_expansion_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (open, _) in input.match_indices('(') {
        let mut headword_start = open;
        while headword_start > 0 && bytes[headword_start - 1].is_ascii_alphabetic() {
            headword_start -= 1;
        }
        let headword = &input[headword_start..open];
        if headword.len() < 2 || !headword.bytes().all(|byte| byte.is_ascii_uppercase()) {
            continue;
        }

        let parenthetical_tail = &input[open + 1..];
        let Some(close) = parenthetical_tail.find(')') else {
            continue;
        };
        let contents = &parenthetical_tail[..close];
        if contents.is_empty()
            || contents.trim_matches(' ') != contents
            || !contents
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || byte == b' ')
        {
            continue;
        }

        if contents.split_ascii_whitespace().count() >= 2 {
            spans.push(InputSpan {
                start_byte: headword_start,
                end_byte: open,
            });
        }
    }
    spans
}

fn has_uppercase_roman_headword_expansion(input: &str) -> bool {
    !uppercase_roman_headword_expansion_spans(input).is_empty()
}

/// Narrows the HCA-style diagnostic to a position-sensitive mode boundary:
/// a preceding whitespace-delimited word contains Roman letters and ends in
/// Korean, followed by an uppercase headword expansion. This identifies a
/// mixed Roman+Korean particle boundary without naming a particular particle.
fn mixed_roman_korean_before_headword_expansion_spans(input: &str) -> Vec<InputSpan> {
    uppercase_roman_headword_expansion_spans(input)
        .into_iter()
        .filter(|span| {
            let before = &input[..span.start_byte];
            if !before.chars().next_back().is_some_and(char::is_whitespace) {
                return false;
            }
            let previous_word = before
                .trim_end_matches(char::is_whitespace)
                .rsplit(char::is_whitespace)
                .next()
                .unwrap_or("");
            previous_word.chars().any(|ch| ch.is_ascii_alphabetic())
                && previous_word.chars().any(is_korean_script)
                && previous_word
                    .chars()
                    .next_back()
                    .is_some_and(is_korean_script)
        })
        .collect()
}

/// Finds a maximal, alphanumeric-delimited ASCII letter run of two or more
/// capitals, excluding the headword of an immediately following parenthetical
/// expansion already covered by `UPPERCASE_ROMAN_HEADWORD_EXPANSION`.
fn has_standalone_uppercase_roman_word(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_alphabetic() {
            cursor += input[cursor..]
                .chars()
                .next()
                .expect("cursor must remain on a character boundary")
                .len_utf8();
            continue;
        }

        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_alphabetic() {
            cursor += 1;
        }
        let run = &input[start..cursor];
        let previous = input[..start].chars().next_back();
        let next = input[cursor..].chars().next();
        let is_alphanumeric_delimited = previous.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && next.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        if run.len() >= 2
            && run.bytes().all(|byte| byte.is_ascii_uppercase())
            && is_alphanumeric_delimited
            && next != Some('(')
        {
            return true;
        }
    }
    false
}

fn is_korean_script(ch: char) -> bool {
    matches!(ch as u32, 0x3131..=0x318e | 0xac00..=0xd7a3)
}

/// Cross-cutting shape shared by prose acronyms and scientific formulae:
/// an immediate Korean prefix followed by a closed, two-or-more-letter
/// all-caps ASCII parenthetical such as `책임자(COO)` or `일산화탄소(CO)`.
fn has_korean_prefixed_allcaps_parenthetical(input: &str) -> bool {
    for (open, _) in input.match_indices('(') {
        if !input[..open]
            .chars()
            .next_back()
            .is_some_and(is_korean_script)
        {
            continue;
        }
        let tail = &input[open + 1..];
        let Some(close) = tail.find(')') else {
            continue;
        };
        let body = &tail[..close];
        if body.len() >= 2 && body.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return true;
        }
    }
    false
}

/// Cross-cutting input-only shape such as `AI·SW`: two maximal ASCII-letter
/// runs of at least two capitals joined directly by U+00B7 MIDDLE DOT.
///
/// This deliberately does not assign prose, mathematics, or science
/// semantics. The 2024 rules use the same character as Korean punctuation
/// and as a multiplication mark, so the shape remains an analyzer cohort.
fn has_allcaps_roman_middle_dot_runs(input: &str) -> bool {
    let bytes = input.as_bytes();
    for (middle_dot, _) in input.match_indices('·') {
        let mut left_start = middle_dot;
        while left_start > 0 && bytes[left_start - 1].is_ascii_alphabetic() {
            left_start -= 1;
        }

        let right_start = middle_dot + '·'.len_utf8();
        let mut right_end = right_start;
        while right_end < bytes.len() && bytes[right_end].is_ascii_alphabetic() {
            right_end += 1;
        }

        let left = &input[left_start..middle_dot];
        let right = &input[right_start..right_end];
        let previous = input[..left_start].chars().next_back();
        let next = input[right_end..].chars().next();
        let has_alphanumeric_boundaries = previous.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && next.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        if left.len() >= 2
            && right.len() >= 2
            && left.bytes().all(|byte| byte.is_ascii_uppercase())
            && right.bytes().all(|byte| byte.is_ascii_uppercase())
            && has_alphanumeric_boundaries
        {
            return true;
        }
    }
    false
}

/// Finds an ASCII-letter run followed immediately by U+00B7 and either the
/// first following Korean character or the complete following ASCII-letter
/// run. The span is syntactic only: it does not infer punctuation, product-name,
/// or mathematical semantics from the middle dot.
fn roman_run_before_middle_dot_boundary_spans(input: &str) -> Vec<InputSpan> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    for (middle_dot, mark) in input.match_indices('·') {
        let mut left_start = middle_dot;
        while left_start > 0 && bytes[left_start - 1].is_ascii_alphabetic() {
            left_start -= 1;
        }
        if left_start == middle_dot
            || input[..left_start]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric())
        {
            continue;
        }

        let right_start = middle_dot + mark.len();
        let Some(first_right) = input[right_start..].chars().next() else {
            continue;
        };
        let right_end = if first_right.is_ascii_alphabetic() {
            let mut end = right_start;
            while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                end += 1;
            }
            end
        } else if is_korean_script(first_right) {
            right_start + first_right.len_utf8()
        } else {
            continue;
        };
        spans.push(InputSpan {
            start_byte: left_start,
            end_byte: right_end,
        });
    }
    spans
}

fn first_difference_at_roman_middle_dot_boundary(item: &EncodedCase) -> bool {
    first_difference_in_korean_context_signature_spans(
        item,
        &roman_run_before_middle_dot_boundary_spans(&item.located.case.input),
        0,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InlineParenthesizedOperator {
    open_byte: usize,
    operator: char,
}

/// Finds `한글(<operator>)한글` spans without assigning a meaning from the
/// corpus reference. The operator set is exactly the arithmetic set named by
/// Hangeul rules 45/46; parentheses remain visible input boundaries.
fn inline_parenthesized_operators(input: &str) -> Vec<InlineParenthesizedOperator> {
    let mut matches = Vec::new();
    for (open_byte, _) in input.match_indices('(') {
        if !input[..open_byte]
            .chars()
            .next_back()
            .is_some_and(is_korean_script)
        {
            continue;
        }

        let tail = &input[open_byte + 1..];
        let Some(operator) = tail.chars().next() else {
            continue;
        };
        if !matches!(
            operator,
            '+' | '-' | '\u{2212}' | '\u{00d7}' | '\u{00f7}' | '='
        ) {
            continue;
        }
        let after_operator = &tail[operator.len_utf8()..];
        let Some(after_close) = after_operator.strip_prefix(')') else {
            continue;
        };
        if after_close.chars().next().is_some_and(is_korean_script) {
            matches.push(InlineParenthesizedOperator {
                open_byte,
                operator,
            });
        }
    }
    matches
}

/// Returns the current engine's actual cell ranges for the detected input
/// structures. This is deliberately derived from encoding a neutral Korean
/// probe rather than from the corpus reference. A range is retained only when
/// the full sentence has the same current-engine signature at the cell offset
/// produced by the prefix ending immediately before `(`.
fn inline_parenthesized_operator_actual_ranges(
    input: &str,
    actual: &str,
) -> Vec<std::ops::Range<usize>> {
    let actual_cells = actual.chars().collect::<Vec<_>>();
    let left_probe_cells = braillify::encode_to_unicode("가")
        .expect("neutral Korean probe must encode")
        .chars()
        .count();
    let right_probe_cells = braillify::encode_to_unicode("나")
        .expect("neutral Korean probe must encode")
        .chars()
        .count();

    inline_parenthesized_operators(input)
        .into_iter()
        .filter_map(|candidate| {
            let prefix = braillify::encode_to_unicode(&input[..candidate.open_byte]).ok()?;
            let start = prefix.chars().count();
            let probe =
                braillify::encode_to_unicode(&format!("가({})나", candidate.operator)).ok()?;
            let probe_cells = probe.chars().collect::<Vec<_>>();
            let end = probe_cells.len().checked_sub(right_probe_cells)?;
            let signature = probe_cells.get(left_probe_cells..end)?;
            let actual_end = start.checked_add(signature.len())?;
            (actual_cells.get(start..actual_end) == Some(signature)).then_some(start..actual_end)
        })
        .collect()
}

fn first_difference_in_inline_parenthesized_operator(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    inline_parenthesized_operator_actual_ranges(&item.located.case.input, actual)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

fn tight_triangle_spans(input: &str) -> Vec<InputSpan> {
    input
        .match_indices('△')
        .filter_map(|(byte, mark)| {
            let next = input[byte + mark.len()..].chars().next()?;
            is_korean_script(next).then_some(InputSpan {
                start_byte: byte,
                end_byte: byte + mark.len() + next.len_utf8(),
            })
        })
        .collect()
}

fn tight_triangle_positions(input: &str) -> Vec<usize> {
    tight_triangle_spans(input)
        .into_iter()
        .map(|span| span.start_byte)
        .collect()
}

/// Current-engine ranges for `△한글`, including the first Korean cell after
/// the mark. A missing reference space therefore differs inside this range,
/// while unrelated earlier sentence differences do not count as causal.
fn tight_triangle_actual_ranges(input: &str, actual: &str) -> Vec<std::ops::Range<usize>> {
    korean_context_signature_ranges(input, actual, &tight_triangle_spans(input), 0)
}

fn first_difference_in_tight_triangle(item: &EncodedCase) -> bool {
    let Ok(actual) = &item.actual else {
        return false;
    };
    if actual == &item.located.case.unicode {
        return false;
    }
    let first_difference = first_difference_cell(&item.located.case.unicode, actual);
    tight_triangle_actual_ranges(&item.located.case.input, actual)
        .into_iter()
        .any(|range| range.contains(&first_difference))
}

fn enum_key<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("enum serialization must succeed")
        .as_str()
        .expect("enum must serialize as a string")
        .to_string()
}

fn excerpt_pair(expected: &str, actual: &str) -> (String, String) {
    let expected_chars = expected.chars().collect::<Vec<_>>();
    let actual_chars = actual.chars().collect::<Vec<_>>();
    let first_diff = first_difference_cell(expected, actual);
    let start = first_diff.saturating_sub(8);
    let expected_excerpt = expected_chars.iter().skip(start).take(24).collect();
    let actual_excerpt = actual_chars.iter().skip(start).take(24).collect();
    (expected_excerpt, actual_excerpt)
}

fn first_difference_cell(expected: &str, actual: &str) -> usize {
    expected
        .chars()
        .zip(actual.chars())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| expected.chars().count().min(actual.chars().count()))
}

fn cell_transition_key(expected: &str, actual: &str, index: usize) -> String {
    let label = |text: &str| {
        text.chars().nth(index).map_or_else(
            || "<end>".to_string(),
            |cell| format!("U+{:04X} {cell}", cell as u32),
        )
    };
    format!("{} -> {}", label(expected), label(actual))
}

fn record_pending_first_difference_transition(
    transitions: &mut BTreeMap<String, FirstDifferenceTransitionStats>,
    item: &EncodedCase,
    primary_key: &str,
    reason_key: &str,
    sample_limit: usize,
) {
    let Ok(actual) = &item.actual else {
        return;
    };
    let expected = &item.located.case.unicode;
    if actual == expected {
        return;
    }
    let first_difference = first_difference_cell(expected, actual);
    let key = cell_transition_key(expected, actual, first_difference);
    let stats = transitions.entry(key).or_default();
    stats.cases += 1;
    if stats.samples.len() >= sample_limit
        || stats
            .samples
            .iter()
            .any(|sample| sample.shard == item.located.shard)
    {
        return;
    }
    let (expected_excerpt, actual_excerpt) = excerpt_pair(expected, actual);
    stats.samples.push(PendingRuleReviewClusterSample {
        shard: item.located.shard.clone(),
        index: item.located.index,
        input: item.located.case.input.clone(),
        expected_excerpt,
        actual_excerpt,
        first_difference_cell: Some(first_difference),
        error: None,
        primary_class: primary_key.to_string(),
        reason: reason_key.to_string(),
    });
}

fn is_compatibility_unit_decomposition(ch: char, nfkc: &str) -> bool {
    matches!(
        ch as u32,
        0x3371..=0x337a
            | 0x3380..=0x33c6
            | 0x33c8..=0x33cc
            | 0x33ce..=0x33d0
            | 0x33d3..=0x33d9
            | 0x33db..=0x33df
            | 0x33ff
    ) && nfkc.chars().any(|part| part.is_ascii_alphabetic())
        && nfkc.chars().all(|part| {
            part.is_ascii_alphabetic()
                || matches!(part, '2' | '3' | '/' | '\u{2044}' | '\u{2215}' | 'μ')
        })
}

fn encoding_error_family(ch: char) -> &'static str {
    let nfkc = ch.to_string().nfkc().collect::<String>();
    if is_compatibility_unit_decomposition(ch, &nfkc) {
        "compatibility_unit_symbol"
    } else if is_roman_numeral_presentation(ch)
        && nfkc.chars().all(|part| {
            matches!(
                part.to_ascii_uppercase(),
                'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'
            )
        })
    {
        "roman_numeral_presentation"
    } else if matches!(ch, '\u{3214}' | '\u{321c}') {
        "enclosed_organization_mark"
    } else if ch == '\u{2113}' {
        "letterlike_unit_symbol"
    } else if matches!(
        ch,
        '\u{02d1}'
            | '\u{2025}'
            | '\u{2502}'
            | '\u{25b2}'
            | '\u{25b4}'
            | '\u{260f}'
            | '\u{2665}'
            | '\u{2e31}'
            | '\u{302e}'
    ) {
        "punctuation_or_layout_symbol"
    } else {
        "other_unsupported_symbol"
    }
}

fn record_structural_cohort_case(
    stats: &mut PendingRuleReviewClusterStats,
    item: &EncodedCase,
    primary_key: &str,
    reason_key: &str,
    sample_limit: usize,
    first_difference_in_output_signature: Option<bool>,
    include_localized_sample_bucket: bool,
) {
    stats.candidates += 1;
    let outcome = if primary_key == "exact" {
        stats.exact += 1;
        "exact"
    } else {
        stats.mismatch += 1;
        if let Some(is_in_signature) = first_difference_in_output_signature {
            stats.output_signature_mismatches_evaluated += 1;
            stats.first_difference_in_output_signature += usize::from(is_in_signature);
            if is_in_signature && let Ok(actual) = &item.actual {
                let expected = &item.located.case.unicode;
                let first_difference = first_difference_cell(expected, actual);
                *stats
                    .first_difference_in_output_signature_transitions
                    .entry(cell_transition_key(expected, actual, first_difference))
                    .or_insert(0) += 1;
            }
        }
        if reason_key == "conflicting_duplicate_reference" {
            stats.conflicting_reference_cases += 1;
        }
        *stats
            .mismatch_primary_classes
            .entry(primary_key.to_string())
            .or_insert(0) += 1;
        "mismatch"
    };
    let expected = &item.located.case.unicode;
    let (actual, error) = match &item.actual {
        Ok(actual) => (actual.as_str(), None),
        Err(error) => ("", Some(error.clone())),
    };
    let (expected_excerpt, actual_excerpt) = if actual == expected {
        (
            expected.chars().take(24).collect(),
            actual.chars().take(24).collect(),
        )
    } else {
        excerpt_pair(expected, actual)
    };
    let first_difference_cell =
        (actual != expected).then(|| first_difference_cell(expected, actual));
    let sample = PendingRuleReviewClusterSample {
        shard: item.located.shard.clone(),
        index: item.located.index,
        input: item.located.case.input.clone(),
        expected_excerpt,
        actual_excerpt,
        first_difference_cell,
        error,
        primary_class: primary_key.to_string(),
        reason: reason_key.to_string(),
    };
    let mut bucket_names = vec![outcome];
    if include_localized_sample_bucket && first_difference_in_output_signature == Some(true) {
        bucket_names.push("localized_mismatch");
    }
    for bucket_name in bucket_names {
        let bucket = stats.samples.entry(bucket_name.to_string()).or_default();
        if bucket.len() < sample_limit
            && !bucket
                .iter()
                .any(|existing| existing.shard == item.located.shard)
        {
            bucket.push(sample.clone());
        }
    }
}

fn analyze(
    cases: Vec<LocatedCase>,
    encoded: Vec<EncodedCase>,
    sample_limit: usize,
) -> AnalysisReport {
    let (duplicate_inputs, conflicting) = conflicting_inputs(&cases);
    let mut primary_classes = BTreeMap::new();
    let mut reasons = BTreeMap::new();
    let mut encoding_error_messages = BTreeMap::new();
    let mut encoding_error_families = BTreeMap::new();
    let mut singleton_error_characters = BTreeMap::<String, ErrorCharacterStats>::new();
    let mut encoding_error_audit = EncodingErrorAudit::default();
    let mut traits = BTreeMap::new();
    let mut shards = BTreeMap::<String, ShardStats>::new();
    let mut samples = BTreeMap::<String, Vec<Sample>>::new();
    let mut rule_36_transition_audit = Rule36TransitionAudit::default();
    let mut pending_rule_review_clusters = BTreeMap::from([
        (
            RULE69_ASCII_UNIT_BEFORE_TERMINATOR_SKIPPING_SYMBOL.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            COMPACT_NUMERIC_ASCII_SUFFIX.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            DECIMAL_POINT_BETWEEN_DIGITS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ALLCAPS_ROMAN_RUN_CONTAINING_OU.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ALLCAPS_ROMAN_MIDDLE_DOT_RUNS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ROMAN_RUN_BEFORE_MIDDLE_DOT_BOUNDARY.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_PREFIXED_CLOSED_ROMAN_ANNOTATION.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_INLINE_PARENTHESIZED_OPERATOR.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            MIXED_ROMAN_KOREAN_BEFORE_HEADWORD_EXPANSION.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            SINGLE_CAPITAL_PARENTHESIZED_DIGITS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            STANDALONE_UPPERCASE_ROMAN_WORD.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            TIGHT_TRIANGLE_BEFORE_KOREAN.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            UPPERCASE_ROMAN_HEADWORD_EXPANSION.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            UPPERCASE_ROMAN_HYPHEN_DIGITS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ALLCAPS_SHORTFORM_PREFIX_COLLISION.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ROMAN_UPPERCASE_AFTER_DIGIT.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ROMAN_UPPERCASE_AFTER_HYPHEN.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            PURE_ALLCAPS_HYPHEN_MULTI_ALLCAPS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ROMAN_HYPHENATED_WORD_AFTER_KOREAN_WORD.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ROMAN_PARENTHETICAL_HEADWORD_AFTER_KOREAN_WORD.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_PREFIXED_ROMAN_PARENTHETICAL_HYPHEN_SUFFIX.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            CONSECUTIVE_ROMAN_UPPERCASE_WORD_REENTRY.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
    ]);
    let mut pending_first_difference_cell_transitions = BTreeMap::new();
    let mut pending_first_difference_transitions_after_localized_cohorts = BTreeMap::new();
    let mut compact_numeric_ascii_suffixes = BTreeMap::new();
    let mut grade1_shortform_prefix_surfaces = BTreeMap::new();
    let mut grade1_numeric_continuation_surfaces = BTreeMap::new();
    let mut grade1_hyphen_continuation_surfaces = BTreeMap::new();
    let mut exact = 0usize;

    for item in &encoded {
        let (primary, reason) = classify(item, &conflicting);
        if let Some(transition) = rule_36_observed_transition(item) {
            rule_36_transition_audit.presentation_cases += 1;
            *rule_36_transition_audit
                .observed_transitions
                .entry(transition.to_string())
                .or_insert(0) += 1;

            if item.actual.is_err() {
                let other_unsupported_characters = item
                    .singleton_unsupported_characters
                    .iter()
                    .copied()
                    .filter(|ch| !is_roman_numeral_presentation(*ch))
                    .map(|ch| format!("U+{:04X} {ch}", ch as u32))
                    .collect::<Vec<_>>();
                rule_36_transition_audit.remaining_complex_errors += 1;
                rule_36_transition_audit
                    .remaining_complex_error_samples
                    .push(Rule36ComplexErrorSample {
                        shard: item.located.shard.clone(),
                        index: item.located.index,
                        input: item.located.case.input.clone(),
                        other_unsupported_characters,
                    });
            }
        }
        let primary_key = enum_key(&primary);
        let reason_key = enum_key(&reason);
        *primary_classes.entry(primary_key.clone()).or_insert(0) += 1;
        *reasons.entry(reason_key.clone()).or_insert(0) += 1;

        if primary == PrimaryClass::PendingRuleReview {
            record_pending_first_difference_transition(
                &mut pending_first_difference_cell_transitions,
                item,
                &primary_key,
                &reason_key,
                sample_limit,
            );
            if !first_difference_claimed_by_localized_cohort(item) {
                record_pending_first_difference_transition(
                    &mut pending_first_difference_transitions_after_localized_cohorts,
                    item,
                    &primary_key,
                    &reason_key,
                    sample_limit,
                );
            }
        }

        for (cluster, present, localized_first_difference, localized_samples) in [
            (
                RULE69_ASCII_UNIT_BEFORE_TERMINATOR_SKIPPING_SYMBOL,
                !rule69_ascii_unit_before_terminator_skipping_symbol_spans(
                    &item.located.case.input,
                )
                .is_empty(),
                Some(first_difference_at_rule69_ascii_unit_terminator_boundary(
                    item,
                )),
                true,
            ),
            (
                COMPACT_NUMERIC_ASCII_SUFFIX,
                !compact_numeric_ascii_suffix_spans(&item.located.case.input).is_empty(),
                Some(first_difference_in_compact_numeric_ascii_suffix(item)),
                true,
            ),
            (
                DECIMAL_POINT_BETWEEN_DIGITS,
                !decimal_word_spans(&item.located.case.input).is_empty(),
                Some(first_difference_in_decimal_word(item)),
                true,
            ),
            (
                ALLCAPS_ROMAN_RUN_CONTAINING_OU,
                !allcaps_roman_runs_containing_ou(&item.located.case.input).is_empty(),
                Some(first_difference_in_allcaps_ou_run(item)),
                false,
            ),
            (
                ALLCAPS_ROMAN_MIDDLE_DOT_RUNS,
                has_allcaps_roman_middle_dot_runs(&item.located.case.input),
                None,
                false,
            ),
            (
                ROMAN_RUN_BEFORE_MIDDLE_DOT_BOUNDARY,
                !roman_run_before_middle_dot_boundary_spans(&item.located.case.input).is_empty(),
                Some(first_difference_at_roman_middle_dot_boundary(item)),
                true,
            ),
            (
                KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL,
                has_korean_prefixed_allcaps_parenthetical(&item.located.case.input),
                None,
                false,
            ),
            (
                KOREAN_PREFIXED_CLOSED_ROMAN_ANNOTATION,
                !korean_prefixed_closed_roman_annotation_spans(&item.located.case.input).is_empty(),
                Some(first_difference_in_korean_prefixed_annotation_opening(item)),
                true,
            ),
            (
                KOREAN_INLINE_PARENTHESIZED_OPERATOR,
                !inline_parenthesized_operators(&item.located.case.input).is_empty(),
                Some(first_difference_in_inline_parenthesized_operator(item)),
                false,
            ),
            (
                MIXED_ROMAN_KOREAN_BEFORE_HEADWORD_EXPANSION,
                !mixed_roman_korean_before_headword_expansion_spans(&item.located.case.input)
                    .is_empty(),
                Some(first_difference_in_signature_spans(
                    item,
                    &mixed_roman_korean_before_headword_expansion_spans(&item.located.case.input),
                    1,
                )),
                false,
            ),
            (
                SINGLE_CAPITAL_PARENTHESIZED_DIGITS,
                !single_capital_parenthesized_digit_spans(&item.located.case.input).is_empty(),
                Some(first_difference_in_signature_spans(
                    item,
                    &single_capital_parenthesized_digit_spans(&item.located.case.input),
                    1,
                )),
                false,
            ),
            (
                STANDALONE_UPPERCASE_ROMAN_WORD,
                has_standalone_uppercase_roman_word(&item.located.case.input),
                None,
                false,
            ),
            (
                TIGHT_TRIANGLE_BEFORE_KOREAN,
                !tight_triangle_positions(&item.located.case.input).is_empty(),
                Some(first_difference_in_tight_triangle(item)),
                false,
            ),
            (
                UPPERCASE_ROMAN_HEADWORD_EXPANSION,
                has_uppercase_roman_headword_expansion(&item.located.case.input),
                None,
                false,
            ),
            (
                UPPERCASE_ROMAN_HYPHEN_DIGITS,
                !uppercase_roman_hyphen_digit_spans(&item.located.case.input).is_empty(),
                Some(first_difference_in_korean_context_signature_spans(
                    item,
                    &uppercase_roman_hyphen_digit_spans(&item.located.case.input),
                    1,
                )),
                false,
            ),
            (
                ALLCAPS_SHORTFORM_PREFIX_COLLISION,
                !allcaps_shortform_prefix_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_by_prior_localized_cohort(item)
                        && first_difference_in_grade1_cohort_spans(
                            item,
                            &allcaps_shortform_prefix_spans(&item.located.case.input),
                        ),
                ),
                true,
            ),
            (
                ROMAN_UPPERCASE_AFTER_DIGIT,
                !roman_uppercase_after_digit_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_by_prior_localized_cohort(item)
                        && first_difference_in_grade1_cohort_spans(
                            item,
                            &roman_uppercase_after_digit_spans(&item.located.case.input),
                        ),
                ),
                true,
            ),
            (
                ROMAN_UPPERCASE_AFTER_HYPHEN,
                !roman_uppercase_after_hyphen_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_by_prior_localized_cohort(item)
                        && first_difference_in_grade1_cohort_spans(
                            item,
                            &roman_uppercase_after_hyphen_spans(&item.located.case.input),
                        ),
                ),
                true,
            ),
            (
                PURE_ALLCAPS_HYPHEN_MULTI_ALLCAPS,
                !pure_allcaps_hyphen_multi_allcaps_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_by_prior_localized_cohort(item)
                        && first_difference_in_grade1_cohort_spans(
                            item,
                            &pure_allcaps_hyphen_multi_allcaps_spans(&item.located.case.input),
                        ),
                ),
                true,
            ),
            (
                ROMAN_HYPHENATED_WORD_AFTER_KOREAN_WORD,
                !roman_hyphenated_word_after_korean_word_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_before_roman_entry_residual(item)
                        && first_difference_at_input_span_entry(
                            item,
                            &roman_hyphenated_word_after_korean_word_spans(
                                &item.located.case.input,
                            ),
                            2,
                        ),
                ),
                true,
            ),
            (
                ROMAN_PARENTHETICAL_HEADWORD_AFTER_KOREAN_WORD,
                !roman_parenthetical_headword_after_korean_word_spans(&item.located.case.input)
                    .is_empty(),
                Some(
                    !first_difference_claimed_before_roman_entry_residual(item)
                        && first_difference_at_input_span_entry(
                            item,
                            &roman_parenthetical_headword_after_korean_word_spans(
                                &item.located.case.input,
                            ),
                            2,
                        ),
                ),
                true,
            ),
            (
                KOREAN_PREFIXED_ROMAN_PARENTHETICAL_HYPHEN_SUFFIX,
                !korean_prefixed_roman_parenthetical_hyphen_suffix_spans(&item.located.case.input)
                    .is_empty(),
                Some(
                    !first_difference_claimed_before_roman_entry_residual(item)
                        && first_difference_at_input_span_entry(
                            item,
                            &korean_prefixed_roman_parenthetical_hyphen_suffix_spans(
                                &item.located.case.input,
                            ),
                            2,
                        ),
                ),
                true,
            ),
            (
                CONSECUTIVE_ROMAN_UPPERCASE_WORD_REENTRY,
                !consecutive_roman_uppercase_word_spans(&item.located.case.input).is_empty(),
                Some(
                    !first_difference_claimed_before_consecutive_roman_reentry(item)
                        && first_difference_in_current_roman_entry_signature(
                            item,
                            &consecutive_roman_uppercase_word_spans(&item.located.case.input),
                        ),
                ),
                true,
            ),
        ] {
            if !present {
                continue;
            }
            let stats = pending_rule_review_clusters
                .get_mut(cluster)
                .expect("registered pending-rule-review cluster must exist");
            record_structural_cohort_case(
                stats,
                item,
                &primary_key,
                &reason_key,
                sample_limit,
                localized_first_difference,
                localized_samples,
            );
        }

        let mut suffix_spans = BTreeMap::<String, Vec<InputSpan>>::new();
        for span in compact_numeric_ascii_suffix_spans(&item.located.case.input) {
            suffix_spans
                .entry(compact_numeric_ascii_suffix(span, &item.located.case.input).to_string())
                .or_default()
                .push(span);
        }
        for (suffix, spans) in suffix_spans {
            let localized = first_difference_in_compact_numeric_ascii_suffix_spans(item, &spans);
            record_structural_cohort_case(
                compact_numeric_ascii_suffixes.entry(suffix).or_default(),
                item,
                &primary_key,
                &reason_key,
                sample_limit,
                Some(localized),
                false,
            );
        }

        for (target, spans) in [
            (
                &mut grade1_shortform_prefix_surfaces,
                allcaps_shortform_prefix_spans(&item.located.case.input),
            ),
            (
                &mut grade1_numeric_continuation_surfaces,
                roman_uppercase_after_digit_spans(&item.located.case.input),
            ),
            (
                &mut grade1_hyphen_continuation_surfaces,
                roman_uppercase_after_hyphen_spans(&item.located.case.input),
            ),
        ] {
            let mut surface_spans = BTreeMap::<String, Vec<InputSpan>>::new();
            for span in spans {
                surface_spans
                    .entry(item.located.case.input[span.start_byte..span.end_byte].to_string())
                    .or_default()
                    .push(span);
            }
            for (surface, matching_spans) in surface_spans {
                let localized = !first_difference_claimed_by_prior_localized_cohort(item)
                    && first_difference_in_grade1_cohort_spans(item, &matching_spans);
                record_structural_cohort_case(
                    target.entry(surface).or_default(),
                    item,
                    &primary_key,
                    &reason_key,
                    sample_limit,
                    Some(localized),
                    true,
                );
            }
        }

        let shard = shards.entry(item.located.shard.clone()).or_default();
        shard.total += 1;
        if primary == PrimaryClass::Exact {
            exact += 1;
            shard.exact += 1;
            continue;
        }

        let input = &item.located.case.input;
        if let Err(error) = &item.actual {
            encoding_error_audit.raw_total += 1;
            if primary == PrimaryClass::ComparisonMethod {
                encoding_error_audit.resolved_by_comparison_method += 1;
            } else if primary == PrimaryClass::CorpusSuspect {
                encoding_error_audit.excluded_as_corpus_suspect += 1;
            } else {
                encoding_error_audit.unresolved_review_total += 1;
                if item.singleton_unsupported_characters.is_empty() {
                    encoding_error_audit.unclassified_without_singleton += 1;
                    if encoding_error_audit.unclassified_samples.len() < sample_limit {
                        encoding_error_audit.unclassified_samples.push(
                            UnclassifiedEncodingErrorSample {
                                shard: item.located.shard.clone(),
                                index: item.located.index,
                                input: item.located.case.input.clone(),
                                error: error.clone(),
                            },
                        );
                    }
                } else {
                    encoding_error_audit.explained_by_singleton_unsupported += 1;
                    if item.singleton_unsupported_characters.len() > 1 {
                        encoding_error_audit.multiple_singleton_unsupported += 1;
                        encoding_error_audit.multiple_singleton_samples.push(
                            MultipleSingletonErrorSample {
                                shard: item.located.shard.clone(),
                                index: item.located.index,
                                input: item.located.case.input.clone(),
                                unsupported_characters: item
                                    .singleton_unsupported_characters
                                    .iter()
                                    .map(|ch| format!("U+{:04X} {ch}", *ch as u32))
                                    .collect(),
                            },
                        );
                    }
                }
                *encoding_error_messages.entry(error.clone()).or_insert(0) += 1;
                let mut case_families = BTreeSet::new();
                for ch in &item.singleton_unsupported_characters {
                    let key = format!("U+{:04X} {ch}", *ch as u32);
                    let family = encoding_error_family(*ch);
                    case_families.insert(family);
                    singleton_error_characters
                        .entry(key)
                        .and_modify(|stats| stats.cases += 1)
                        .or_insert_with(|| ErrorCharacterStats {
                            cases: 1,
                            nfkc: ch.to_string().nfkc().collect(),
                            family,
                        });
                }
                for family in case_families {
                    *encoding_error_families
                        .entry(family.to_string())
                        .or_insert(0) += 1;
                }
            }
        }
        for (name, present) in [
            (
                "contains_ascii_letters",
                input.chars().any(|ch| ch.is_ascii_alphabetic()),
            ),
            (
                "contains_ascii_digits",
                input.chars().any(|ch| ch.is_ascii_digit()),
            ),
            (
                "contains_delimiter_or_quote",
                input.chars().any(is_delimiter_or_quote),
            ),
            (
                "contains_non_ascii_whitespace",
                input.chars().any(|ch| ch.is_whitespace() && ch != ' '),
            ),
            ("input_not_nfc", input.nfc().ne(input.chars())),
            ("input_not_nfkc", input.nfkc().ne(input.chars())),
        ] {
            if present {
                *traits.entry(name.to_string()).or_insert(0) += 1;
            }
        }

        let bucket = samples.entry(reason_key).or_default();
        if bucket.len() < sample_limit {
            let (actual_excerpt, error) = match &item.actual {
                Ok(actual) => (actual.as_str(), None),
                Err(error) => ("", Some(error.clone())),
            };
            let (expected_excerpt, actual_excerpt) =
                excerpt_pair(&item.located.case.unicode, actual_excerpt);
            bucket.push(Sample {
                shard: item.located.shard.clone(),
                index: item.located.index,
                input: input.clone(),
                expected_excerpt,
                actual_excerpt,
                error,
            });
        }
    }

    let total = encoded.len();
    AnalysisReport {
        corpus: "NIKL Korean-Korean Braille Parallel Corpus 2025 v1.0",
        total,
        exact,
        mismatch: total - exact,
        exact_percent: exact as f64 / total as f64 * 100.0,
        duplicate_inputs,
        conflicting_duplicate_inputs: conflicting.len(),
        primary_classes,
        reasons,
        encoding_error_messages,
        encoding_error_families,
        singleton_error_characters,
        encoding_error_audit,
        rule_36_transition_audit,
        pending_rule_review_clusters,
        pending_first_difference_cell_transitions,
        pending_first_difference_transitions_after_localized_cohorts,
        compact_numeric_ascii_suffixes,
        grade1_shortform_prefix_surfaces,
        grade1_numeric_continuation_surfaces,
        grade1_hyphen_continuation_surfaces,
        overlapping_traits: traits,
        shards,
        samples,
    }
}

fn markdown(report: &AnalysisReport) -> String {
    let mut text = String::new();
    text.push_str("# NIKL 2025 v1.0 corpus analysis\n\n");
    text.push_str(
        "> Generated by `cargo run --release -p braillify --example nikl_corpus_analyze`. \
         The tool reads only `input` and `unicode`; it never loads or compares the read-only \
         `world` or `jeomsarang` fields.\n\n",
    );
    text.push_str("## Current measurement\n\n");
    text.push_str("| Metric | Count |\n|---|---:|\n");
    text.push_str(&format!("| Total | {} |\n", report.total));
    text.push_str(&format!("| Exact | {} |\n", report.exact));
    text.push_str(&format!("| Mismatch | {} |\n", report.mismatch));
    text.push_str(&format!(
        "| Exact accuracy | {:.2}% |\n",
        report.exact_percent
    ));
    text.push_str(&format!(
        "| Duplicate records | {} |\n",
        report.duplicate_inputs
    ));
    text.push_str(&format!(
        "| Inputs with conflicting references | {} |\n\n",
        report.conflicting_duplicate_inputs
    ));

    text.push_str("## Classification policy\n\n");
    text.push_str(
        "Primary classes are evidence gates, not permissions to change the engine. \
         `implementation_defect` is restricted to defects independently confirmed from the PDF \
         (currently the rules 28/29 roman-indicator ordering signature). \
         `unsupported_character_review` contains encoding failures fully explained by one or more \
         singleton characters whose support obligation has not been confirmed from the PDF. \
         `unclassified_encoding_error_review` contains other encoding failures until a PDF-backed \
         implementation obligation or a reproducible comparison/corpus issue is established. \
         `pending_rule_review` contains foreign-text, number, punctuation, and Korean candidates \
         that have not yet been resolved against the PDF. `corpus_suspect` is reserved for \
         independently detectable contradictions such as one input having multiple references. \
         `comparison_method` requires equality after a named normalization.\n\n",
    );
    text.push_str("| Primary class | Count |\n|---|---:|\n");
    for (name, count) in &report.primary_classes {
        text.push_str(&format!("| `{name}` | {count} |\n"));
    }
    text.push_str("\n| Reproducible reason | Count |\n|---|---:|\n");
    for (name, count) in &report.reasons {
        text.push_str(&format!("| `{name}` | {count} |\n"));
    }

    text.push_str("\n## Pending first-difference cell transitions\n\n");
    text.push_str(
        "This ranking is a diagnostic selector, not an implementation rule. It counts only \
         current `pending_rule_review` cases whose encoder call succeeded, keyed by the expected \
         and actual cell at the sentence's first differing position. Candidate implementation \
         work must still bind a transition to a localized input structure, exact controls, and \
         independent PDF evidence.\n\n",
    );
    let mut ranked_transitions = report
        .pending_first_difference_cell_transitions
        .iter()
        .collect::<Vec<_>>();
    ranked_transitions.sort_by(|(left_key, left), (right_key, right)| {
        right
            .cases
            .cmp(&left.cases)
            .then_with(|| left_key.cmp(right_key))
    });
    text.push_str("| Rank | Expected → actual first cell | Cases |\n|---:|---|---:|\n");
    for (rank, (transition, stats)) in ranked_transitions.iter().take(20).enumerate() {
        text.push_str(&format!(
            "| {} | `{transition}` | {} |\n",
            rank + 1,
            stats.cases
        ));
    }
    for (transition, stats) in ranked_transitions.iter().take(10) {
        text.push_str(&format!("\n### `{transition}`\n\n"));
        for sample in &stats.samples {
            text.push_str(&format!(
                "- `{}` #{}: {}\n  - expected: `{}`\n  - actual: `{}`\n  - first differing cell (zero-based): {}\n  - current primary/reason: `{}` / `{}`\n",
                sample.shard,
                sample.index,
                sample
                    .input
                    .chars()
                    .take(180)
                    .collect::<String>()
                    .replace('`', "\\`"),
                sample.expected_excerpt,
                sample.actual_excerpt,
                sample.first_difference_cell.unwrap_or(0),
                sample.primary_class,
                sample.reason
            ));
        }
    }

    text.push_str("\n## Residual first-difference transitions after localized cohorts\n\n");
    text.push_str(
        "This ranking removes only cases whose first difference is inside an existing \
         output-localized cohort. Broad input-only traits are not exclusion masks. The residual \
         table therefore prioritizes new causes without hiding a mismatch merely because an \
         unrelated structure coexists elsewhere in its sentence.\n\n",
    );
    let mut residual_transitions = report
        .pending_first_difference_transitions_after_localized_cohorts
        .iter()
        .collect::<Vec<_>>();
    residual_transitions.sort_by(|(left_key, left), (right_key, right)| {
        right
            .cases
            .cmp(&left.cases)
            .then_with(|| left_key.cmp(right_key))
    });
    text.push_str("| Rank | Expected → actual first cell | Residual cases |\n|---:|---|---:|\n");
    for (rank, (transition, stats)) in residual_transitions.iter().take(20).enumerate() {
        text.push_str(&format!(
            "| {} | `{transition}` | {} |\n",
            rank + 1,
            stats.cases
        ));
    }
    for (transition, stats) in residual_transitions.iter().take(10) {
        text.push_str(&format!("\n### Residual `{transition}`\n\n"));
        for sample in &stats.samples {
            text.push_str(&format!(
                "- `{}` #{}: {}\n  - expected: `{}`\n  - actual: `{}`\n  - first differing cell (zero-based): {}\n  - current primary/reason: `{}` / `{}`\n",
                sample.shard,
                sample.index,
                sample
                    .input
                    .chars()
                    .take(180)
                    .collect::<String>()
                    .replace('`', "\\`"),
                sample.expected_excerpt,
                sample.actual_excerpt,
                sample.first_difference_cell.unwrap_or(0),
                sample.primary_class,
                sample.reason
            ));
        }
    }

    text.push_str("\n## Cross-cutting input-only structural cohorts\n\n");
    text.push_str(
        "These are cross-cutting input-only structural cohorts, not new primary classes and not \
         engine routing rules. Candidate selection never changes a case's existing primary \
         class by itself. Only cohort members already classified as `pending_rule_review` form a pending \
         subcluster; exact and other-primary members are controls that retain their existing \
         outcomes. A separate classifier may use independently justified, output-localized PDF \
         evidence, as in the rule-34 three-cell contradiction below. The \
         `uppercase_roman_headword_closed_multiword_parenthetical` gate requires a two-or-more \
         character uppercase ASCII headword immediately followed by a closed parenthesis whose \
         contents are two or more ASCII Roman words separated only by spaces. Because the \
         contents admit only letters and spaces, visible operators, subscript/superscript \
         notation, and nested parentheses are excluded deterministically. The \
         `mixed_roman_korean_word_before_uppercase_headword_expansion` gate further requires a \
         whitespace-delimited preceding word that contains both ASCII Roman and Korean and ends \
         in Korean. It locates the following uppercase headword itself, including its immediately \
         preceding emitted cell, so an earlier Roman entry in the same sentence cannot satisfy \
         the output audit. The `single_capital_followed_by_parenthesized_digits` gate requires a \
         standalone capital, a non-empty closed ASCII-digit parenthetical, and alphanumeric outer \
         boundaries. It likewise includes the emitted entry-boundary cell in localization. The \
         `uppercase_roman_run_followed_by_hyphen_digits` gate requires a maximal uppercase ASCII \
         run, literal ASCII hyphen-minus, a non-empty digit run, and alphanumeric outer \
         boundaries. Its localized range includes the current encoded run and its immediately \
         preceding cell, keeping `F-35`-style routing separate from both parenthesized digits and \
         headword expansions. The \
         `standalone_multi_character_uppercase_roman_word` gate finds maximal ASCII-letter runs \
         of two or more capitals with non-alphanumeric boundaries; a run immediately followed \
         by `(` is excluded so the HCA-style headword itself is not counted by both gates. The \
         `korean_prefixed_closed_allcaps_parenthetical` gate requires an immediately preceding \
         Korean character and a closed body of two or more uppercase ASCII letters. It \
         intentionally contains both acronym annotations (`책임자(COO)`) and scientific \
         formulae (`일산화탄소(CO)`) so their semantic collision remains measurable. The \
         `korean_prefixed_closed_roman_annotation_rule_34_order` gate accepts the narrower \
         rule-34 body grammar after an immediately preceding Korean character and localizes only \
         the current engine's Korean opening-parenthesis cells. This distinguishes the PDF's \
         parenthesis-before-Roman-indicator order from unrelated differences later in the same \
         sentence. The \
         `multi_character_allcaps_roman_runs_joined_by_middle_dot` gate requires two maximal \
         ASCII-letter runs of at least two capitals joined directly by U+00B7, with \
         non-alphanumeric outer boundaries. It records shapes such as `AI·SW` without assigning \
         prose, mathematics, or science semantics. The \
         `roman_run_immediately_before_attached_middle_dot_boundary` gate is output-localized: \
         it requires a maximal ASCII-letter run immediately before U+00B7 and an attached \
         Korean character or ASCII-letter run after it, then searches for that whole \
         current-engine signature in the actual output. It therefore isolates the Roman \
         terminator boundary without treating unrelated middle dots elsewhere in the sentence \
         as causal. The \
         `korean_inline_parenthesized_single_arithmetic_operator` gate requires an immediate \
         `Korean(` + one rule-45 arithmetic operator + `)Korean` span. Unlike broad coexistence \
         traits, it also locates the current engine's emitted structure and counts a mismatch as \
         signature-local only when the sentence's first differing cell falls inside that output \
         range. The `allcaps_roman_run_containing_ou` gate finds maximal, alphanumeric-delimited \
         uppercase ASCII runs containing adjacent `OU`. It locates the independently encoded run \
         signature in the complete current output and counts only first differences inside that \
         signature as localized. The `decimal_point_between_ascii_digits` gate finds \
         whitespace-delimited words containing `digit.digit` and reproduces each whole word in a \
         neutral Korean context, so suffixes and punctuation remain part of the current-engine \
         signature. The `compact_numeric_ascii_letter_suffix` gate finds a numeric prefix \
         immediately followed by ASCII letters, includes the immediately preceding output cell \
         as its entry boundary, and retains suffix-specific outcome counts. It \
         intentionally includes both possible rule-69 units and ambiguous variable/identifier \
         forms; membership alone does not assign unit semantics. The \
         `rule69_ascii_unit_before_terminator_skipping_symbol` gate is narrower: it accepts only \
         ASCII unit spellings already supported by rule 69, includes the immediately following \
         rule-33/34 punctuation cell in the localized signature, and does not infer new units. The \
         `tight_triangle_mark_immediately_before_korean` gate requires literal \
         `△한글` with no input space and includes the first following Korean cell in its localized \
         output range, so an observed missing-space difference is measured at the mark boundary.\n\n",
    );
    text.push_str(
        "| Cluster | Candidates | Exact | Mismatch | Conflicting-reference cases |\n\
         |---|---:|---:|---:|---:|\n",
    );
    for (name, stats) in &report.pending_rule_review_clusters {
        text.push_str(&format!(
            "| `{name}` | {} | {} | {} | {} |\n",
            stats.candidates, stats.exact, stats.mismatch, stats.conflicting_reference_cases
        ));
    }
    for (name, stats) in &report.pending_rule_review_clusters {
        text.push_str(&format!("\n### `{name}`\n\n"));
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "Of the {} candidates, {pending} are the actual `pending_rule_review` subcluster. \
             The other {} candidates are exact or existing non-pending-primary controls; this \
             membership alone does not reclassify them.\n\n",
            stats.candidates,
            stats.candidates - pending
        ));
        if stats.output_signature_mismatches_evaluated > 0 {
            text.push_str(&format!(
                "For this output-signature audit, {} mismatches were evaluable and {} have their \
                 first differing cell inside the detected structure's current-engine output \
                 range. The remaining mismatches are controls against causal over-attribution.\n\n",
                stats.output_signature_mismatches_evaluated,
                stats.first_difference_in_output_signature
            ));
            if !stats
                .first_difference_in_output_signature_transitions
                .is_empty()
            {
                let mut transitions = stats
                    .first_difference_in_output_signature_transitions
                    .iter()
                    .collect::<Vec<_>>();
                transitions.sort_by(|(left_key, left), (right_key, right)| {
                    right.cmp(left).then_with(|| left_key.cmp(right_key))
                });
                text.push_str("Localized first-difference transitions:\n\n");
                for (transition, count) in transitions.into_iter().take(5) {
                    text.push_str(&format!("- `{transition}`: {count}\n"));
                }
                text.push('\n');
            }
        }
        text.push_str("Mismatch primary-class distribution:\n\n");
        for (primary, count) in &stats.mismatch_primary_classes {
            text.push_str(&format!("- `{primary}`: {count}\n"));
        }
        for (outcome, samples) in &stats.samples {
            text.push_str(&format!("\nRepresentative `{outcome}` samples:\n\n"));
            for sample in samples {
                text.push_str(&format!(
                    "- `{}` #{}: {}\n  - expected: `{}`\n  - actual: `{}`{}{}\n  - current primary/reason: `{}` / `{}`\n",
                    sample.shard,
                    sample.index,
                    sample
                        .input
                        .chars()
                        .take(180)
                        .collect::<String>()
                        .replace('`', "\\`"),
                    sample.expected_excerpt,
                    sample.actual_excerpt,
                    sample
                        .error
                        .as_ref()
                        .map_or_else(String::new, |error| format!("\n  - error: `{error}`")),
                    sample.first_difference_cell.map_or_else(String::new, |cell| {
                        format!("\n  - first differing cell (zero-based): {cell}")
                    }),
                    sample.primary_class,
                    sample.reason
                ));
            }
        }
    }
    text.push_str("\n## UEB grade-1 first-difference cohorts\n\n");
    text.push_str(
        "These cohorts are defined by both an input boundary and the sentence's actual \
         first-difference transition. They therefore do not claim every mismatch merely \
         coexisting with an ASCII run. Candidate, exact, mismatch, and primary-class counts \
         remain cross-cutting controls; only the reported target transition is the localized \
         residual under review. The reverse transition is retained separately rather than \
         folded into the target.\n\n",
    );
    text.push_str(
        "| Cohort | Candidates | Exact controls | Mismatch | Target localized | Reverse |\n\
         |---|---:|---:|---:|---:|---:|\n",
    );
    for (name, target, reverse) in [
        (
            ALLCAPS_SHORTFORM_PREFIX_COLLISION,
            "U+2830 ⠰ -> U+2820 ⠠",
            "U+2820 ⠠ -> U+2830 ⠰",
        ),
        (
            ROMAN_UPPERCASE_AFTER_DIGIT,
            "U+2820 ⠠ -> U+2830 ⠰",
            "U+2830 ⠰ -> U+2820 ⠠",
        ),
        (
            ROMAN_UPPERCASE_AFTER_HYPHEN,
            "U+2820 ⠠ -> U+2830 ⠰",
            "U+2830 ⠰ -> U+2820 ⠠",
        ),
        (
            PURE_ALLCAPS_HYPHEN_MULTI_ALLCAPS,
            "U+2820 ⠠ -> U+2830 ⠰",
            "U+2830 ⠰ -> U+2820 ⠠",
        ),
    ] {
        let stats = report
            .pending_rule_review_clusters
            .get(name)
            .expect("registered grade-1 cohort must exist");
        let target_count = stats
            .first_difference_in_output_signature_transitions
            .get(target)
            .copied()
            .unwrap_or(0);
        let reverse_count = stats
            .first_difference_in_output_signature_transitions
            .get(reverse)
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "| `{name}` | {} | {} | {} | {target_count} | {reverse_count} |\n",
            stats.candidates, stats.exact, stats.mismatch
        ));
    }
    text.push_str(
        "\n### All-caps shortform prefix at an attached Roman entry\n\n\
         UEB 2024 rule 5.7.2 requires grade-1 mode when a letters-sequence could \
         be read as a shortform or as containing one. Rule 10.9.7 covers a \
         standing-alone shortform-shaped sequence, rule 10.9.8 covers a sequence at \
         the beginning of a longer word (the PDF example is `LLC`), and rule 5.8.1 \
         places grade 1 before capitalization. The current standalone ASCII-token \
         route already supplies that guard for complete shortform-shaped controls \
         such as `AC`, `CD`, `IMM`, and `AG`; the attached Korean-word/parenthetical \
         route enters directly at the capital marker and accounts for part of the \
         localized `⠰ -> ⠠` signature. This is a routing distinction supported \
         independently by the PDF, not an expected-output lookup. The implemented \
         boundary is only rule 10.9.7's complete pure-letter shortform. Longer runs \
         such as `GDP`, `LLM`, and the PDF's rule-10.9.8 `LLC` example remain in the \
         broad diagnostic cohort but are not generalized in Korean routing: that \
         broader experiment regressed its exact controls. A shortform appearing \
         later would require the still-distinct grade-1 word rule 10.9.9.\n\n\
         Implementation-boundary experiment (all numbers are full-corpus exact \
         matches, with the committed analyzer-only checkpoint as baseline):\n\n\
         | Boundary | Exact / 83,528 | Change | Decision |\n\
         |---|---:|---:|---|\n\
         | Analyzer-only baseline | 66,546 | — | control |\n\
         | Prefix guard extended through the uppercase token route | 65,264 | -1,282 | rejected |\n\
         | Same-token token route narrowed, rule-28 prefix retained | 65,355 | -1,191 | rejected |\n\
         | Uppercase token route restored, rule-28 prefix retained | 65,474 | -1,072 | rejected |\n\
         | Rule-28 complete shortform only | 66,683 | +137 | retained |\n\n\
         At the retained boundary the broad cohort moves from 2,239 exact / 1,881 \
         mismatch / 962 target-localized / 1 reverse to 2,376 exact / 1,744 \
         mismatch / 778 target-localized / 42 reverse. These figures do not turn \
         the remaining longer-prefix members into an engine rule; they preserve \
         the failed broader trials as evidence that input shape alone is unsafe.\n\n",
    );
    text.push_str(
        "Same-surface controls demonstrate why primary classes must not be changed \
         by cohort membership:\n\n\
         | Surface | Candidates | Exact | Mismatch | Target-localized |\n\
         |---|---:|---:|---:|---:|\n",
    );
    for surface in ["AC", "LLM", "CD", "IMM", "AG", "GDP", "WD"] {
        if let Some(stats) = report.grade1_shortform_prefix_surfaces.get(surface) {
            let localized = stats
                .first_difference_in_output_signature_transitions
                .get("U+2830 ⠰ -> U+2820 ⠠")
                .copied()
                .unwrap_or(0);
            text.push_str(&format!(
                "| `{surface}` | {} | {} | {} | {localized} |\n",
                stats.candidates, stats.exact, stats.mismatch
            ));
        }
    }
    for surface in ["AC", "LLM", "CD"] {
        let Some(stats) = report.grade1_shortform_prefix_surfaces.get(surface) else {
            continue;
        };
        let exact = stats
            .samples
            .get("exact")
            .and_then(|samples| samples.first());
        let localized = stats
            .samples
            .get("localized_mismatch")
            .and_then(|samples| samples.first());
        if let (Some(exact), Some(localized)) = (exact, localized) {
            text.push_str(&format!(
                "\n- `{surface}` exact control: `{}` #{} — {}\n\
                 - `{surface}` localized mismatch: `{}` #{} — {}\n",
                exact.shard,
                exact.index,
                exact.input.chars().take(180).collect::<String>(),
                localized.shard,
                localized.index,
                localized.input.chars().take(180).collect::<String>()
            ));
        }
    }
    text.push_str(
        "\n### Uppercase immediately after a digit\n\n\
         UEB rule 5.6.1 says the numeric indicator establishes grade-1 mode, and \
         rule 5.6.2 ends that mode at a space, hyphen, dash, or grade-1 terminator; \
         a capitalization indicator is not a terminator. Korean rule 35 likewise \
         keeps Roman letters and an adjacent number in one Roman section. The PDF's \
         printed `3b`, `3B`, and `3m` examples distinguish the three following-letter \
         classes: lowercase `a`-`j` retains `⠰` because its cells are numeric, a \
         capital uses its capitalization indicator, and lowercase `k`-`z` needs no \
         extra indicator. `Braille4All`, `M4G`, and `W1N` independently confirm the \
         capital boundary inside longer alphanumeric strings. Before the engine \
         change this cohort contained 330 localized `⠠ -> ⠰` cases. A blanket \
         digit-to-letter removal reached 67,000/83,528 (+317) but was rejected: \
         retaining `⠰` only for lowercase `a`-`j` recovers 10 exact cases and raises \
         the result to 67,010. The wrapper control also exposes a separate routing \
         boundary: a numeric run already preceded by an ASCII letter is part of the \
         Roman identifier, not a fresh rule-69 compact unit. Preserving the rule-69 \
         path for genuinely numeric-leading units while excluding that identifier \
         boundary adds 2 more exact cases, for a final 67,012 (+329). The uppercase \
         cohort moves from 756 exact / 1,140 mismatch / 330 target-localized / 1 \
         reverse to 1,078 exact / 818 mismatch / 0 target-localized / 1 reverse. The remaining \
         non-exact members are not attributed to the removed uppercase transition: \
         their sentence-level first difference may lie in another structure and \
         remains under its existing primary class. This numeric state change remains \
         separate from both the complete-shortform guard and the hyphen continuation \
         boundary below.\n\n\
         ### Uppercase immediately after a hyphen\n\n\
         UEB rule 5.7.2 prints `CD-ROM` with one grade-1 indicator before `CD` and \
         no second grade-1 indicator after the hyphen. Korean rule 29 similarly \
         uses one Roman span for consecutive Roman text. Before the engine change, \
         the broad diagnostic contained 952 candidates / 32 exact / 920 mismatch, \
         with 312 localized `⠠ -> ⠰` and 2 reverse transitions. A blanket \
         uppercase-suffix removal reached 67,222 (+210) but made the broad cohort's \
         single-capital controls such as `Around-U`, `DALL-E`, `ISMS-P`, and `USB-C` \
         non-exact; it was rejected. Requiring only a two-letter uppercase suffix \
         reached 67,162 (+150) but regressed the mixed-prefix exact control `Ko-LLM`; \
         it was also rejected. The retained boundary matches the complete PDF shape: \
         the immediately adjacent prefix is a pure-uppercase letter segment and the \
         immediately adjacent suffix is a pure-uppercase segment of at least two \
         letters. It reaches 67,138 (+126) while preserving all 32 baseline exact \
         controls. The broad diagnostic now contains 952 candidates / 158 exact / \
         794 mismatch, with 157 localized `⠠ -> ⠰` and 3 reverse transitions. The \
         dedicated `pure_allcaps_segment_before_hyphen_and_multi_allcaps_segment_after` \
         row reports only the implemented subset; broad mixed-case and single-capital \
         members remain controls or pending review. `K-ALM` is the one new reverse \
         surface but was already a mismatch before this change, not an exact \
         regression. Digit-hyphen forms such as `F-35` remain excluded, and the \
         complete-shortform guard still legitimately precedes `CD` in `CD-ROM`.\n\n",
    );
    text.push_str("\n## Roman-entry residual cohorts after grade-1 localization\n\n");
    text.push_str(
        "These three cohorts split the former dominant `⠴ -> blank` residual by the input \
         structure at the actual first-difference location. Their entry boundary is anchored by \
         independently encoding the input prefix and requiring it to equal the full current-engine \
         output prefix; repeated Roman text elsewhere cannot satisfy the locator. A localized count \
         is recorded only when no earlier output-localized cohort already claims that first \
         difference. Candidate membership remains cross-cutting and never changes a primary class.\n\n\
         Korean rule 29 requires a Roman indicator before Roman text in a Korean sentence. Rules \
         33-35 define the relevant hyphen, enclosure, and number boundaries. Independently, math \
         rules 2, 6, 11, 12, and 45 permit subtraction, parentheses, Roman variables, and function \
         notation with overlapping ASCII surface forms. The surface gates below therefore cannot \
         by themselves exclude a mathematical reading.\n\n",
    );
    text.push_str(
        "| Cohort | Candidates | Exact controls | Mismatch | Pending | Corpus suspect | \
         Localized `⠴ -> blank` | Reverse `blank -> ⠴` |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for name in [
        ROMAN_HYPHENATED_WORD_AFTER_KOREAN_WORD,
        ROMAN_PARENTHETICAL_HEADWORD_AFTER_KOREAN_WORD,
        KOREAN_PREFIXED_ROMAN_PARENTHETICAL_HYPHEN_SUFFIX,
    ] {
        let stats = report
            .pending_rule_review_clusters
            .get(name)
            .expect("registered Roman-entry cohort must exist");
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        let corpus_suspect = stats
            .mismatch_primary_classes
            .get("corpus_suspect")
            .copied()
            .unwrap_or(0);
        let target = stats
            .first_difference_in_output_signature_transitions
            .get("U+2834 ⠴ -> U+2800 ⠀")
            .copied()
            .unwrap_or(0);
        let reverse = stats
            .first_difference_in_output_signature_transitions
            .get("U+2800 ⠀ -> U+2834 ⠴")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "| `{name}` | {} | {} | {} | {pending} | {corpus_suspect} | {target} | \
             {reverse} |\n",
            stats.candidates, stats.exact, stats.mismatch
        ));
    }
    let parenthetical = report
        .pending_rule_review_clusters
        .get(ROMAN_PARENTHETICAL_HEADWORD_AFTER_KOREAN_WORD)
        .expect("registered parenthetical-headword cohort must exist");
    let capital_to_roman = parenthetical
        .first_difference_in_output_signature_transitions
        .get("U+2820 ⠠ -> U+2834 ⠴")
        .copied()
        .unwrap_or(0);
    let roman_to_grade1 = parenthetical
        .first_difference_in_output_signature_transitions
        .get("U+2834 ⠴ -> U+2830 ⠰")
        .copied()
        .unwrap_or(0);
    text.push_str(&format!(
        "\nThe whitespace parenthetical-headword cohort also retains {capital_to_roman} localized \
         `⠠ -> ⠴` and {roman_to_grade1} localized `⠴ -> ⠰` cases as separate transitions; they \
         are not folded into the target. The exact controls demonstrate that the broad structure \
         is already correct in many sentences, while the attached parenthetical-hyphen cohort \
         has no exact control and includes cases already classified by the stricter rule-34 \
         reference-order contradiction. Consequently none of these measurements authorizes an \
         engine change; they are deterministic pending/corpus-review diagnostics only.\n\n"
    ));
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(CONSECUTIVE_ROMAN_UPPERCASE_WORD_REENTRY)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        let target = stats
            .first_difference_in_output_signature_transitions
            .get("U+2820 ⠠ -> U+2834 ⠴")
            .copied()
            .unwrap_or(0);
        let grade1_to_roman = stats
            .first_difference_in_output_signature_transitions
            .get("U+2830 ⠰ -> U+2834 ⠴")
            .copied()
            .unwrap_or(0);
        let reverse = stats
            .first_difference_in_output_signature_transitions
            .get("U+2834 ⠴ -> U+2820 ⠠")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "### Consecutive Roman uppercase-word re-entry\n\n\
             Korean rule 29 explicitly says that when two or more Roman items occur \
             consecutively, the Roman indicator is placed only before the first and the \
             terminator only after the last. Its printed `Los Angeles` and `Table of Contents` \
             examples exercise multiword Roman sections; the rule-28 appendix independently \
             supplies capitalization indicators inside that section. The current token phase can \
             nevertheless insert an explicit Roman-entry event before a later uppercase word when \
             the preceding Roman run began inside a mixed Korean/punctuation word. The character \
             emitter is still in Roman mode at that point, so this is a candidate duplicate-event \
             boundary rather than permission to rewrite arbitrary multiword ASCII text.\n\n\
             Baseline measurement: {} candidates, {} exact controls, {} mismatches, {pending} \
             pending members, and {}/{} evaluable mismatches localized to the current re-entry \
             signature. The localized transitions are {target} `⠠ -> ⠴`, \
             {grade1_to_roman} `⠰ -> ⠴`, and {reverse} reverse `⠴ -> ⠠`; the remaining localized \
             transitions stay separate. Exact controls include contexts where the first Roman word \
             already opened token-level mode, while parenthesized/mixed-token examples expose the \
             duplicate event. Any engine experiment must therefore suppress only an explicit entry \
             encountered while final emit state is already Roman, then audit all exact regressions \
             and the complete 5,141-case standard suite.\n\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    text.push_str(
        "\nThe HCA-style headword-expansion shape described above is not an engine \
         implementation premise. The 2024 PDF's math rule 6 \
         defines parentheses and grouping parentheses, rule 11 defines mathematical-expression \
         spacing, rule 12 covers Roman letters in formulas as well as Korean sentences, and \
         rule 45 shows Roman-letter function notation followed by parentheses. Excluding visible \
         operators, scripts, and nesting narrows this corpus cohort, but the PDF does not make \
         the remaining surface shape sufficient to rule out every mathematical counterexample. \
         The cluster therefore remains conservative pending-review evidence only.\n\n\
         The standalone-uppercase cohort is similarly ambiguous. Hangeul rule 28's appendix \
         defines the capital-word indicator for two or more consecutive capitals, and rule 29 \
         defines Roman indicators around Roman text in a Korean sentence. But math rule 12 also \
         uses uppercase Roman variables, while science rule 7 uses uppercase runs in chemical \
         formulas. The input gate cannot determine which semantic regime applies, so its output \
         differences are observations to review, not permission to infer an engine rule from the \
         corpus reference.\n\n\
         The Korean-prefixed all-caps parenthetical cohort isolates that ambiguity more narrowly. \
         Hangeul rules 28/29 require Roman and capitalization indicators for prose acronyms, \
         while science rule 7 requires element-by-element capitals for chemical formulae. Both \
         meanings can have the same input surface form. The observed `COO`/`NSC`/`MOU` output \
         differences therefore do not justify disabling either algorithm without independent \
         semantic evidence.\n\n\
         Two narrower cohorts separate causes hidden by the frequent `U+2834 -> U+2800` cell \
         transition. `single_capital_followed_by_parenthesized_digits` reproduces the current \
         math-token routing of forms such as `A(14)`: Hangeul rules 29 and 34 govern a Roman \
         section and a parenthesized Roman form, while math rule 6 independently defines \
         parenthesized function notation such as `f(x)`. A capital and numeric argument do not \
         remove that mathematical counterexample, so this localized routing difference remains \
         pending rather than authorizing an input-shape exception. \
         `mixed_roman_korean_word_before_uppercase_headword_expansion` separately targets the \
         next Roman headword after a mixed Roman+Korean word (for example, a Korean particle \
         attached to the previous Roman name). Its range is anchored to that later headword, not \
         to the earlier Roman entry. Nevertheless, the closed multiword parenthetical shape still \
         cannot exclude every mathematical interpretation under math rules 6, 11, 12, and 45, \
         as recorded for the broader HCA-style cohort. The headword shape is therefore not added \
         to engine routing; the two causes and their controls remain separately measurable.\n\n\
         The uppercase-Roman hyphen-digits cohort is a third independent cause. Hangeul rule 35 \
         explicitly shows `D-100` as a Roman-and-number continuation (2024 Korean-rules PDF \
         p.29), while math rule 2 defines subtraction and the math chapters allow uppercase Roman \
         variables. The surface form alone therefore does not prove whether `F-35` is an \
         identifier or a subtraction expression. This cohort records the current operator-routing \
         signature and exact controls without merging it into either `A(14)` or HCA-style \
         diagnostics. No engine change is made without both a safe semantic boundary and exact \
         controls.\n\n\
         The all-caps `OU` cohort isolates a frequent output transition without treating the \
         reference as a rule. Hangeul rules 28, 29, and 32 delegate Roman-letter content to UEB \
         (2024 Korean-rules PDF p.25 and following rules). \
         UEB 10.12.1 says not to use a contraction when it is known or can be determined that an \
         abbreviation or acronym's letters are pronounced separately, but says to use the \
         contraction when that pronunciation is in doubt; UEB 10.12.2 otherwise uses \
         contractions in abbreviations and acronyms (UEB 2024 PDF pp.191-192; Korean UEB \
         translation PDF pp.182-183). Thus an expected `o` + `u` versus the \
         current `ou` groupsign can be localized to an uppercase run, yet the surface run alone \
         cannot distinguish a letter-by-letter initialism from a pronounceable word or acronym. \
         That distinction needs lexical or semantic evidence absent from this input gate. Exact \
         members are controls, identical-input conflicting references remain `corpus_suspect`, \
         and no engine change is inferred.\n\n\
         The all-caps Roman middle-dot cohort is also semantically underdetermined. Hangeul rule \
         29 defines Roman indicators around Roman text in a Korean sentence, and Hangeul rule 50 \
         requires U+00B7 to be attached on both sides, but neither rule says that the punctuation \
         joins the adjacent Roman runs into one Roman span. Math rule 2 separately defines the \
         same printed dot as multiplication, and science rule 4 uses it inside chemical \
         formulae. An input-only `AI·SW` gate therefore cannot prove which mode transition is \
         required. Exact cases remain controls, mismatches retain their existing primary class, \
         and no engine rule is inferred from their references. Representative samples are \
         sentence-level evidence: when the reported first difference precedes the detected \
         middle-dot span, the cohort must not be treated as the cause of that mismatch.\n\n\
         The narrower Roman-before-middle-dot boundary cohort separates that semantic question \
         from a checkable indicator boundary. Hangeul rule 29 requires a Roman terminator after \
         Roman text. Rule 33 enumerates the punctuation that suppresses or moves that terminator, \
         but does not include U+00B7; rule 50 requires the middle dot to be attached on both sides \
         and does not state a Roman-terminator exception. Thus a localized reference that omits \
         the terminator conflicts with the current rule-29/33 path on the available PDF text. \
         This is conservative corpus/PDF-reference review evidence, not permission to remove the \
         terminator or to reclassify non-localized cases.\n\n\
         The inline parenthesized-operator cohort has an independently checkable spacing boundary. \
         Hangeul rule 46 inserts spaces only when an operation or comparison sign is between \
         Korean text, while the literal parentheses intervene in this gate. Hangeul rule 49 says \
         punctuation spacing follows the print, and science rule 21 prints and brailles `(-)` and \
         `(+)` with no spaces inside the parentheses. The output-signature count is therefore the \
         implementation-candidate subset; mere sentence-level coexistence is retained only as a \
         control. ASCII hyphen-minus is independently supported through the `Symbol`/rule-49 \
         punctuation path, while `+`, `×`, `÷`, and `=` reach the `MathSymbol` spacing rule; \
         end-to-end tests preserve tight parentheses on both paths. Analyzer checkpoint `30a9b10` \
         recorded the pre-fix baseline as 23 candidates, 0 exact, 23 mismatches, and 19 \
         mismatches whose first difference was signature-local. The generalized rule-46/49 fix \
         is evaluated below against that immutable baseline rather than inferred from a reference \
         string.\n\n\
         The tight-triangle cohort is not an implementation premise. Hangeul rule 49 assigns `△` \
         the omission-mark role and requires print spacing to be followed, while rule 72 also \
         assigns the same glyph a bullet role but shows a print space after every bullet. A tight \
         corpus input does not identify which role was intended, and adding a space absent from \
         the input would contradict rule 49 unless independent layout evidence establishes a \
         bullet. The localizer searches the complete actual output for a neutral-Korean, \
         current-engine signature covering the mark and its first following Korean character; \
         it neither encodes a context-sensitive sentence prefix in isolation nor reads the \
         reference output. Tight marks followed by ASCII letters or digits remain outside this \
         gate. Localized mismatches are therefore corpus/layout review evidence only.\n\n\
         Corpus contradictions remain a separate gate: identical inputs with conflicting \
         references are classified as `corpus_suspect` before these cohorts are recorded and \
         would appear explicitly in each mismatch primary-class distribution. Their absence does \
         not prove a reference correct; it only means that this deterministic contradiction test \
         did not fire.\n",
    );
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(UPPERCASE_ROMAN_HYPHEN_DIGITS)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent uppercase-Roman hyphen-digits measurement: {} candidates, {} exact \
             controls, {} mismatches, {pending} members in the actual `pending_rule_review` \
             subcluster, and {}/{} evaluable mismatches whose first difference is inside the \
             target run plus its entry boundary. It remains distinct from parenthesized digits \
             and headword expansions; no engine change is inferred.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(SINGLE_CAPITAL_PARENTHESIZED_DIGITS)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent single-capital parenthesized-digits measurement: {} candidates, {} \
             exact controls, {} mismatches, {pending} members in the actual \
             `pending_rule_review` subcluster, and {}/{} evaluable mismatches whose first \
             difference is inside the target run plus its entry boundary. No engine change is \
             inferred from the ambiguous prose/function surface form.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(MIXED_ROMAN_KOREAN_BEFORE_HEADWORD_EXPANSION)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent mixed Roman+Korean boundary before uppercase headword-expansion \
             measurement: {} candidates, {} exact controls, {} mismatches, {pending} members in \
             the actual `pending_rule_review` subcluster, and {}/{} evaluable mismatches whose \
             first difference is localized to the later headword's entry boundary/output. The \
             detector cannot be satisfied by the earlier Roman entry. No HCA-shaped engine \
             routing rule is introduced.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(COMPACT_NUMERIC_ASCII_SUFFIX)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent compact numeric+ASCII-suffix measurement: {} candidates, {} exact \
             controls, {} mismatches, {pending} members in the actual `pending_rule_review` \
             subcluster, and {}/{} evaluable mismatches whose first difference is inside the \
             complete current-engine output signature or its immediate entry boundary. Rule 40 \
             requires the numeric indicator and rule 69 requires Roman indicators around a \
             Roman-written unit, but the input \
             shape alone cannot prove that every ASCII suffix is a unit.\n\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));

        let mut suffixes = report
            .compact_numeric_ascii_suffixes
            .iter()
            .collect::<Vec<_>>();
        suffixes.sort_by(|(left_key, left), (right_key, right)| {
            right
                .candidates
                .cmp(&left.candidates)
                .then_with(|| left_key.cmp(right_key))
        });
        text.push_str(
            "| ASCII suffix | Candidates | Exact | Mismatch | Localized first diff |\n\
             |---|---:|---:|---:|---:|\n",
        );
        for (suffix, suffix_stats) in suffixes.into_iter().take(25) {
            text.push_str(&format!(
                "| `{suffix}` | {} | {} | {} | {} |\n",
                suffix_stats.candidates,
                suffix_stats.exact,
                suffix_stats.mismatch,
                suffix_stats.first_difference_in_output_signature
            ));
        }
        text.push_str(
            "\nEntry-boundary pre-fix baseline: 2,975 candidates, 1,649 exact controls, 1,326 \
             mismatches, 1,259 pending members, and 356 localized first differences; the \
             dominant reference number-sign versus current space transition accounted for 296 \
            cases. Rules 68 and 69 already accept semantic Unicode compatibility-unit forms. \
             The implementation derives compact ASCII spellings only from their all-letter NFKC \
             decompositions, reuses the owning rule's PDF-defined cells, chooses the longest \
             complete spelling, and rejects partial suffix matches. It does not extend \
             recognition to separated English words or arbitrary corpus suffixes. The same \
             cohort now has 1,759 exact controls, 1,216 mismatches, 1,148 pending members, and \
             261 localized first differences; corpus-wide exact matches moved from 66,436 to \
             66,546 (+110). Rule 69's printed `160㎎/㎗` example directly controls `160mg`; the \
             additional full-encoder `240mg`/`240㎎` pair proves that recognition is invariant \
             to the numeric value and also passes. Rule 68's printed `10,000㎡는 1㏊이다` example \
             controls the `ha` spelling through U+33CA's NFKC decomposition; `15.2ha`/`15.2㏊` \
             now passes without a spelling-specific output branch. The 53-case `ha` suffix \
             control moved from 0 exact to 18 exact; its remaining 35 mismatches have independent \
             later or surrounding differences. A full U+3300..U+33FF owner audit found 73 \
             Rules 68/69 glyphs with pure-ASCII NFKC decompositions, 73 distinct spellings, and \
             therefore no current duplicate-spelling owner collision. Production nevertheless \
             groups every owner before resolution and excludes a spelling if any owner cells \
             differ; a synthetic collision test proves that this is not first-wins behavior. An \
             exhaustive test also compares every one of the 73 derived spellings with every \
             owner glyph at both the unit-cell and full-encoder boundaries in a neutral Korean \
             measurement context. That audit exposed \
             the pre-existing explicit `cal` mapping's missing ordinary terminator; rule 69 \
             requires the terminator, while the existing slash-boundary function removes it for \
             the printed `cal/㎠/min` context. Restoring the ordinary terminator and extending \
             that slash-continuation boundary made the audit pass without changing the \
             corpus-wide 66,546 exact total. Ambiguous pure-English inputs remain on UEB: the \
             standard controls `3m` and `4.m` are retained and pass, rather than being globally \
             forced into a Korean unit route.\n",
        );
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(RULE69_ASCII_UNIT_BEFORE_TERMINATOR_SKIPPING_SYMBOL)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent rule-69 ASCII-unit punctuation-boundary measurement: {} candidates, {} \
             exact controls, {} mismatches, {pending} members in the actual \
             `pending_rule_review` subcluster, and {}/{} evaluable mismatches whose first \
             difference is localized to the unit-plus-punctuation output signature. PDF rule 69 \
             requires a Roman terminator after a Roman-written unit in the ordinary case, while \
             rules 33/34 omit it at the listed punctuation or enclosing-mark boundary; the rule \
             46 PDF example `체중(kg)` is the minimal parenthesized-unit control. Membership is \
             restricted to rule-69 spellings already supported by the engine and does not infer \
             unit semantics for arbitrary ASCII suffixes.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
        text.push_str(
            " Analyzer pre-fix baseline: 440 candidates, 0 exact controls, 440 mismatches, 435 \
             pending members, and 9 signature-local first differences. After the generalized \
             rule-33/34 boundary override and the matching non-math routing guard, the same cohort \
             has 325 exact controls and 115 mismatches. Corpus-wide exact matches moved from \
             66,039 to 66,436 (+397); the additional gains are applications of the same boundary \
             rule outside this strict ASCII detector, including compatibility-unit forms. The \
             complete standard suite remains 5,141/5,141.\n",
        );
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(DECIMAL_POINT_BETWEEN_DIGITS)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent decimal-point measurement: {} candidates, {} exact controls, {} \
             mismatches, {pending} members in the actual `pending_rule_review` subcluster, and \
             {}/{} evaluable mismatches whose first difference is inside the complete \
             decimal-containing word's current-engine signature. Hangeul rules 43 and 48 keep \
             an ASCII point between digits in the numeric sequence and encode it as the decimal \
             point; rules 35 and 69 supply controls for adjacent Roman-number chains and Roman \
             units. This is an implementation-candidate audit, not permission to specialize on \
             a corpus reference.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
        text.push_str(
            "Analyzer checkpoint `ed20f99` recorded the pre-fix baseline as 4,546 candidates, \
             2,584 exact controls, 1,962 mismatches, 1,905 pending members, and 875 localized \
             first differences. Its dominant localized transition was reference decimal point \
             `U+2832 ⠲` versus current Roman indicator `U+2834 ⠴` in 647 cases. After the \
             generalized rule-43/48 guard, that transition is absent: 525 cases become exact \
             and the other corrected prefixes expose later independent mismatches.\n",
        );
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(ALLCAPS_ROMAN_RUN_CONTAINING_OU)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent all-caps `OU` measurement: {} candidates, {} exact controls, {} \
             mismatches, {pending} members in the actual `pending_rule_review` subcluster, and \
             {}/{} evaluable mismatches whose first difference is inside the current-engine \
             output signature for the detected run. This is a pronunciation-sensitive UEB \
             review cohort, not an engine routing rule.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(STANDALONE_UPPERCASE_ROMAN_WORD)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent standalone-uppercase measurement: {} candidates, {} exact controls, {} \
             mismatches, and {pending} members in the actual `pending_rule_review` subcluster. \
             Its high frequency does not make it causal: the same input shape is exact in many \
             cases, and a sentence containing the shape may first differ at another Roman, \
             numeric, or punctuation structure. No engine change is inferred from this cohort.\n",
            stats.candidates, stats.exact, stats.mismatch
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent Korean-prefixed all-caps parenthetical measurement: {} candidates, {} \
             exact controls, {} mismatches, and {pending} members in the actual \
             `pending_rule_review` subcluster. This is a semantic-collision audit, not an engine \
             routing rule; no implementation change is inferred from its reference outputs.\n",
            stats.candidates, stats.exact, stats.mismatch
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(KOREAN_PREFIXED_CLOSED_ROMAN_ANNOTATION)
    {
        let corpus_suspect = stats
            .mismatch_primary_classes
            .get("corpus_suspect")
            .copied()
            .unwrap_or(0);
        let opposite_order = stats
            .first_difference_in_output_signature_transitions
            .get("U+2834 ⠴ -> U+2826 ⠦")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent rule-34 opening-order measurement: {} structural candidates, {} exact \
             controls, {} mismatches, and {}/{} evaluable mismatches whose first difference is \
             inside the current engine's Korean opening-parenthesis cells. Only the \
             {opposite_order} localized first-cell transitions have the reference/current order \
             `⠴` versus `⠦`. After requiring the complete reference `⠴⠐⠣` versus current/PDF \
             `⠦⠄⠴` three-cell signature and preserving higher-priority comparison \
             classifications, {corpus_suspect} are classified as `corpus_suspect`; mere \
             coexistence with a Korean-prefixed Roman annotation does not change a primary \
             class.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(ALLCAPS_ROMAN_MIDDLE_DOT_RUNS)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent all-caps Roman middle-dot measurement: {} candidates, {} exact controls, \
             {} mismatches, and {pending} members in the actual `pending_rule_review` subcluster. \
             This cross-cutting cohort preserves every primary class and is not an engine routing \
             rule.\n",
            stats.candidates, stats.exact, stats.mismatch
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(ROMAN_RUN_BEFORE_MIDDLE_DOT_BOUNDARY)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent Roman-before-middle-dot boundary measurement: {} candidates, {} exact \
             controls, {} mismatches, {pending} members in the actual `pending_rule_review` \
             subcluster, and {}/{} evaluable mismatches whose first difference is localized to \
             the attached Roman/middle-dot output signature. Rules 29, 33, and 50 support the \
             current terminator path but do not support the localized reference omission; no \
             engine change or primary-class rewrite is inferred.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(KOREAN_INLINE_PARENTHESIZED_OPERATOR)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent inline parenthesized-operator measurement: {} candidates, {} exact \
             controls, {} mismatches, {pending} members in the actual `pending_rule_review` \
             subcluster, and {}/{} evaluable mismatches whose first differing cell is inside the \
             emitted structure. Primary classes are preserved.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
        text.push_str(
            " At that implementation checkpoint, the strict cohort moved from 0 to 17 exact \
             cases; the corpus-wide total moved from 65,491 to 65,514 (+23 exact) because the same \
             PDF-backed spacing rule also applied outside the stricter Korean-boundary audit \
             gate. These are immutable checkpoint counts rather than the report's later cumulative \
             total. The complete standard suite remained 5,141/5,141.\n",
        );
    }
    if let Some(stats) = report
        .pending_rule_review_clusters
        .get(TIGHT_TRIANGLE_BEFORE_KOREAN)
    {
        let pending = stats
            .mismatch_primary_classes
            .get("pending_rule_review")
            .copied()
            .unwrap_or(0);
        text.push_str(&format!(
            "\nCurrent tight-triangle measurement: {} candidates, {} exact controls, {} \
             mismatches, {pending} members in the actual `pending_rule_review` subcluster, and \
             {}/{} evaluable mismatches whose first difference is inside the `△` plus first-Korean \
             output range. No engine change is inferred.\n",
            stats.candidates,
            stats.exact,
            stats.mismatch,
            stats.first_difference_in_output_signature,
            stats.output_signature_mismatches_evaluated
        ));
    }

    text.push_str("\n## Encoding-error diagnostics\n\n");
    text.push_str(
        "The audit starts from all raw encoding errors, then separates cases already resolved by \
         a comparison method or corpus contradiction. The message, family, and singleton tables \
         below count only unresolved encoding-error review cases. These diagnostics are not \
         additional primary classes. \
         A singleton unsupported character is a character that also fails when encoded by itself. \
         Such a failure remains a review candidate until the PDF independently establishes support.\n\n",
    );
    text.push_str("| Encoding-error audit | Cases |\n|---|---:|\n");
    text.push_str(&format!(
        "| Raw encoding errors | {} |\n",
        report.encoding_error_audit.raw_total
    ));
    text.push_str(&format!(
        "| Resolved by comparison method | {} |\n",
        report.encoding_error_audit.resolved_by_comparison_method
    ));
    text.push_str(&format!(
        "| Excluded as corpus suspect | {} |\n",
        report.encoding_error_audit.excluded_as_corpus_suspect
    ));
    text.push_str(&format!(
        "| Unresolved encoding-error review cases | {} |\n",
        report.encoding_error_audit.unresolved_review_total
    ));
    text.push_str(&format!(
        "| Explained by singleton unsupported character(s) | {} |\n",
        report
            .encoding_error_audit
            .explained_by_singleton_unsupported
    ));
    text.push_str(&format!(
        "| Multiple singleton unsupported characters | {} |\n",
        report.encoding_error_audit.multiple_singleton_unsupported
    ));
    text.push_str(&format!(
        "| Unclassified without a singleton explanation | {} |\n\n",
        report.encoding_error_audit.unclassified_without_singleton
    ));
    for sample in &report.encoding_error_audit.multiple_singleton_samples {
        text.push_str(&format!(
            "- compound `{}` #{}: {}\n  - singleton unsupported: `{}`\n",
            sample.shard,
            sample.index,
            sample.input.chars().take(180).collect::<String>(),
            sample.unsupported_characters.join(", ")
        ));
    }
    for sample in &report.encoding_error_audit.unclassified_samples {
        text.push_str(&format!(
            "- unclassified `{}` #{}: {} (`{}`)\n",
            sample.shard,
            sample.index,
            sample.input.chars().take(180).collect::<String>(),
            sample.error
        ));
    }
    if !report
        .encoding_error_audit
        .multiple_singleton_samples
        .is_empty()
        || !report.encoding_error_audit.unclassified_samples.is_empty()
    {
        text.push('\n');
    }
    text.push_str("| Error message | Cases |\n|---|---:|\n");
    for (name, count) in &report.encoding_error_messages {
        text.push_str(&format!("| `{name}` | {count} |\n"));
    }
    text.push_str("\n| Error family | Cases |\n|---|---:|\n");
    for (name, count) in &report.encoding_error_families {
        text.push_str(&format!("| `{name}` | {count} |\n"));
    }
    text.push_str(
        "\nFamilies are diagnostics, not automatic normalization permissions. \
         Rules 68/69 compatibility-unit support removed that error family from the current run; \
         `enclosed_organization_mark` and layout symbols still have no confirmed rule.\n\n",
    );
    text.push_str(
        "| Singleton error character | Cases containing it | NFKC decomposition | Family |\n\
         |---|---:|---|---|\n",
    );
    for (name, stats) in &report.singleton_error_characters {
        text.push_str(&format!(
            "| `{name}` | {} | `{}` | `{}` |\n",
            stats.cases,
            stats.nfkc.replace('`', "\\`"),
            stats.family
        ));
    }

    text.push_str("\n## Shards\n\n| Shard | Exact | Total | Accuracy |\n|---|---:|---:|---:|\n");
    for (name, stats) in &report.shards {
        text.push_str(&format!(
            "| `{name}` | {} | {} | {:.2}% |\n",
            stats.exact,
            stats.total,
            stats.exact as f64 / stats.total as f64 * 100.0
        ));
    }

    text.push_str("\n## Overlapping mismatch traits\n\n| Trait | Count |\n|---|---:|\n");
    for (name, count) in &report.overlapping_traits {
        text.push_str(&format!("| `{name}` | {count} |\n"));
    }

    text.push_str("\n## Samples\n\n");
    for (reason, samples) in &report.samples {
        text.push_str(&format!("### `{reason}`\n\n"));
        for sample in samples {
            let input = sample.input.chars().take(180).collect::<String>();
            text.push_str(&format!(
                "- `{}` #{}: {}\n  - expected: `{}`\n  - actual: `{}`{}\n",
                sample.shard,
                sample.index,
                input.replace('`', "\\`"),
                sample.expected_excerpt,
                sample.actual_excerpt,
                sample
                    .error
                    .as_ref()
                    .map_or_else(String::new, |error| format!("\n  - error: `{error}`"))
            ));
        }
        text.push('\n');
    }

    text.push_str("## PDF-derived state gates\n\n");
    text.push_str(
        "The rule 37 example `그는 Can you help me?라고 도움을 요청했다.` distinguishes the first \
         word after the roman indicator (`Can`, whose whole-word sign is suppressed) from the \
         interior word `you` in the uninterrupted ASCII phrase (whose UEB wordsign is retained). \
         The `prev_is_ascii_word && next_is_ascii_word` gate expresses that phrase-interior \
         position rather than matching an input string.\n\n\
         The rule 39 example `What is 김치 in English?` resumes the surrounding English passage \
         after the Korean span. The `english_dominant_wrap_active` gate therefore retains the UEB \
         wordsign for the resumed `in`, instead of treating it as a fresh rule 37 entry word.\n\n",
    );

    text.push_str("## Rule 69 compatibility-unit scope\n\n");
    text.push_str(
        "The engine accepts 96 scientific/measurement glyphs from Unicode CJK Compatibility, \
         derives their Roman spelling with NFKC, and applies rules 68/69 rather than whole-word \
         UEB. The accepted glyph set and panic-free encoding property are fixed by inline tests. \
         The official Unicode names distinguish `U+337A ㍺` SQUARE IU (accepted) from \
         `U+33D1 ㏑` SQUARE LN, `U+33D2 ㏒` SQUARE LOG, and `U+33DA ㏚` SQUARE PR \
         (not units, rejected). See the \
         [Unicode CJK Compatibility names list](https://www.unicode.org/charts/nameslist/n_3300.html).\n\n",
    );

    text.push_str("## Rule 36 Roman-numeral presentation forms\n\n");
    text.push_str(
        "Rule 36 says that a Roman numeral is written with the corresponding Roman letters. \
         The encoder therefore applies compatibility decomposition only to Unicode Roman \
         Numerals U+2160–U+217F and sends the ASCII spelling through the existing rule-36 \
         algorithm. Encoder regressions compare Unicode presentations with ASCII equivalents \
         in the PDF sentence and in attached-Korean, particle-adjacent, and lower-case contexts. \
         U+2180 `ↀ` and unrelated NFKC characters such as `㈜` are explicit non-targets.\n\n",
    );
    text.push_str(
        "The transition audit reconstructs the immediately preceding engine behavior: direct \
         and NFC encoding rejected U+2160–U+217F, while the analyzer's existing NFKC comparison \
         path already used the same ASCII Roman spelling. This avoids a saved-output lookup and \
         keeps the transition reproducible from the current corpus.\n\n",
    );
    text.push_str(&format!(
        "Presentation-form cases audited: {}.\n\n",
        report.rule_36_transition_audit.presentation_cases
    ));
    text.push_str("| Previous observation → current observation | Cases |\n|---|---:|\n");
    for (transition, count) in &report.rule_36_transition_audit.observed_transitions {
        text.push_str(&format!("| `{transition}` | {count} |\n"));
    }
    text.push_str(&format!(
        "\nRemaining complex encoding errors: {}. These cases still contain another character \
         that fails independently, so disappearance of the `roman_numeral_presentation` family \
         does not imply that every former error case now encodes successfully.\n\n",
        report.rule_36_transition_audit.remaining_complex_errors
    ));
    for sample in &report
        .rule_36_transition_audit
        .remaining_complex_error_samples
    {
        let input = sample.input.chars().take(180).collect::<String>();
        let unsupported = if sample.other_unsupported_characters.is_empty() {
            "none detected".to_string()
        } else {
            sample.other_unsupported_characters.join(", ")
        };
        text.push_str(&format!(
            "- `{}` #{}: {}\n  - other independently unsupported: `{}`\n",
            sample.shard,
            sample.index,
            input.replace('`', "\\`"),
            unsupported
        ));
    }
    text.push('\n');

    text.push_str("## Rules 34/54 Korean-prefixed Roman annotations\n\n");
    text.push_str(
        "Rule 34 says that when Roman text is enclosed by quotation marks or brackets, the \
         Roman terminator is omitted; its PDF example is `링컨(Lincoln)은 미국의 제16대 \
         대통령이다.` The example's cells put the printed Korean opening parenthesis \
         (`⠦⠄`) before the Roman indicator (`⠴`). Rule 54 says that text immediately after an opening bracket and \
         immediately before a closing bracket is attached. Together these establish the \
         Korean-prefix + closed-Roman-annotation context independently of corpus expected \
         values. A following comma or period is outside the already closed annotation and \
         must not cause its Roman contents to be rerouted as mathematics.\n\n\
         The implementation gate exists only inside `split_mixed_math_word`, after the prefix \
         has been proved entirely Korean. It accepts a fully closed parenthesized Roman word \
         (including ASCII digits such as `O4O`) plus ordinary trailing prose punctuation. \
         The corpus audit treats the opposite localized reference prefix (`⠴⠐⠣`, Roman \
         indicator plus UEB opening parenthesis) as a data-reference contradiction only when \
         all three cells and the real input position agree. Broad sentence-level coexistence is \
         retained as an exact or existing-primary control. This audit does not alter engine \
         routing. The global math detector is byte-for-byte unchanged; regression tests preserve its \
         existing standalone results for `(x)`, `(A)`, and `(abc)`, while explicit forms such \
         as `(x+1)`, `(a/b)`, and `(x₁)` remain math candidates.\n\n\
         Against the immediately preceding 63,399-exact run, exact matches increased by 2,092. \
         The observable primary totals changed as follows: `comparison_method` 290→303, \
         `pending_rule_review` 19,636→17,543, and `unsupported_character_review` 203→191. \
         Raw encoding errors stayed at 450; errors resolved by a comparison method changed \
         247→259 and unresolved review errors changed 203→191.\n\n",
    );

    text.push_str("## Rule evidence and change log\n\n");
    text.push_str(
        "| Stage | Standard cases | Corpus exact | Corpus accuracy | Evidence |\n\
         |---|---:|---:|---:|---|\n\
         | Parent commit `3cfeae0` | 5,141/5,141 | 57,732/83,528 | 69.12% | Reproduced with release tests |\n\
         | Rules 28/29 indicator ordering | 5,141/5,141 | 61,652/83,528 | 73.81% | Roman indicator now precedes UEB grade-1/capital indicators; rule 35 roman-number continuity retained |\n\
         | Rules 37/39 shared UEB groupsign algorithm | 5,141/5,141 | 63,239/83,528 | 75.71% | Korean Roman sections reuse UEB preference/morphology rules; entry wordsigns and English-dominant resume are state-gated |\n",
    );
    text.push_str(
        "| Rules 68/69 compatibility unit algorithm | 5,141/5,141 | 63,388/83,528 | 75.89% | 96 Unicode unit presentation forms use one decomposition/letter-run/attachment algorithm; encoding errors fell from 434 to 226 |\n",
    );
    text.push_str(
        "| Rule 36 Unicode Roman-numeral presentation normalization | 5,141/5,141 | 63,399/83,528 | 75.90% | U+2160–U+217F use the corresponding Roman-letter spelling; 11 NFKC-equivalent observations became exact, 23 errors became encoded mismatches pending review, and 3 remain blocked by `㈜` |\n",
    );
    text.push_str(
        "| Rules 34/54 Korean-prefixed closed Roman annotation routing | 5,141/5,141 | 65,491/83,528 | 78.41% | A fully closed Roman annotation after an all-Korean prefix stays on the prose encoder path, including attached comma/period and alphanumeric forms such as `O4O`; exact matches increased by 2,092 while the global math detector remained unchanged |\n",
    );
    text.push_str(
        "| Rules 43/48 decimal-point ownership | 5,141/5,141 | 66,039/83,528 | 79.06% | A period directly between ASCII digits remains on the numeric punctuation path even when its word or sentence also contains Roman text; 647 localized Roman-entry differences were removed and 525 cases became exact |\n",
    );
    text.push_str(
        "| Rules 33/34/69 Roman-unit punctuation boundary | 5,141/5,141 | 66,436/83,528 | 79.54% | Rule-69 units retain their ordinary terminator at end/Korean/slash boundaries but omit it before rule-33/34 punctuation or enclosing marks; compact unit tokens with that boundary stay off the math path; 397 cases became exact |\n",
    );
    text.push_str(
        "| Rules 68/69 compact compatibility-derived ASCII units | 5,141/5,141 | 66,546/83,528 | 79.67% | Compact ASCII unit spellings are derived from the engine's already accepted Unicode compatibility-unit forms and reuse their owning-rule cells, with longest-complete matching and no expansion to separated English words; `160mg`, numeric-invariance control `240mg`, and Rule-68 `ha` controls are retained; 110 cases became exact |\n",
    );
    text.push_str(
        "\nThe latest full `cargo test -p braillify test_by_testcase --release -- --nocapture` \
         run was accepted from its custom testcase summary, not the trailing filtered harness: \
         `총 테스트 케이스: 5141`, `성공: 5141`, `실패: 0`, and \
         `Skip (limitation): 0`.\n\n\
         Engine changes must add a row only after both the 5,141-case standard suite and \
         this full analysis have been rerun. Suspect-reference clusters stay in this report; \
         they are not engine targets without independent PDF evidence.\n",
    );
    text
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let config = Config::parse()?;
    let cases = load_cases()?;
    let encoded = encode_cases(&cases, config.threads);
    let report = analyze(cases, encoded, config.sample_limit);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize analysis JSON: {error}"))?;
    write_file(&config.json_path, &json)?;
    write_file(&config.report_path, &markdown(&report))?;
    println!(
        "NIKL corpus: {}/{} exact ({:.2}%), wall={:.3}s, report={}, json={}",
        report.exact,
        report.total,
        report.exact_percent,
        started.elapsed().as_secs_f64(),
        config.report_path.display(),
        config.json_path.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("nikl_corpus_analyze: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case::zero_shards(0, 0, Some("no NIKL corpus shards"))]
    #[case::zero_cases(1, 0, Some("zero cases"))]
    #[case::nonempty_corpus(4, 83_528, None)]
    fn corpus_shape_must_be_nonempty(
        #[case] shard_count: usize,
        #[case] case_count: usize,
        #[case] expected_error: Option<&str>,
    ) {
        let result = validate_corpus_shape(shard_count, case_count);
        match expected_error {
            Some(expected) => assert!(result.unwrap_err().contains(expected)),
            None => assert_eq!(result, Ok(())),
        }
    }

    #[test]
    fn singleton_cache_probes_each_distinct_corpus_character_once() {
        let cases = ["㈜Aℓ", "㈜Bℓ"]
            .into_iter()
            .enumerate()
            .map(|(index, input)| LocatedCase {
                shard: "synthetic.json".to_string(),
                index,
                case: CorpusCase {
                    input: input.to_string(),
                    unicode: String::new(),
                },
            })
            .collect::<Vec<_>>();
        let mut calls = BTreeMap::<char, usize>::new();

        let unsupported = singleton_unsupported_set_with(&cases, |ch| {
            *calls.entry(ch).or_insert(0) += 1;
            matches!(ch, '㈜' | 'ℓ')
        });

        assert_eq!(unsupported, BTreeSet::from(['ℓ', '㈜']));
        assert_eq!(calls.len(), 4);
        assert!(calls.values().all(|count| *count == 1));
    }

    #[rstest::rstest]
    #[case::compatibility_unit('㎏', "compatibility_unit_symbol")]
    #[case::roman_numeral('Ⅱ', "roman_numeral_presentation")]
    #[case::company_mark('㈜', "enclosed_organization_mark")]
    #[case::layout_symbol('▲', "punctuation_or_layout_symbol")]
    #[case::non_unit_square_log('㏒', "other_unsupported_symbol")]
    fn clusters_encoding_error_characters(#[case] input: char, #[case] expected_family: &str) {
        assert_eq!(encoding_error_family(input), expected_family);
    }

    #[rstest::rstest]
    #[case::nfkc_comparison_becomes_exact(
        "Ⅲ",
        "same",
        Some("same"),
        "same",
        &[],
        "nfkc_input_equivalent -> exact"
    )]
    #[case::encoding_error_becomes_pending(
        "Ⅳ장",
        "expected",
        Some("different"),
        "different",
        &[],
        "encoding_error -> encoded_mismatch_pending_rule_review"
    )]
    #[case::compound_encoding_error_remains(
        "Ⅳ㈜",
        "expected",
        None,
        "different",
        &['㈜'],
        "encoding_error -> unsupported_character_review"
    )]
    fn reconstructs_rule_36_observed_transition(
        #[case] input: &str,
        #[case] expected: &str,
        #[case] actual: Option<&str>,
        #[case] nfkc_actual: &str,
        #[case] singleton_unsupported_characters: &[char],
        #[case] expected_transition: &str,
    ) {
        let encoded = EncodedCase {
            located: LocatedCase {
                shard: "synthetic.json".to_string(),
                index: 1,
                case: CorpusCase {
                    input: input.to_string(),
                    unicode: expected.to_string(),
                },
            },
            actual: actual.map_or_else(
                || Err("another unsupported symbol".to_string()),
                |value| Ok(value.to_string()),
            ),
            nfc_actual: None,
            nfkc_actual: Some(Ok(nfkc_actual.to_string())),
            singleton_unsupported_characters: singleton_unsupported_characters.to_vec(),
        };

        assert_eq!(
            rule_36_observed_transition(&encoded),
            Some(expected_transition)
        );
    }

    #[rstest::rstest]
    #[case::singleton_explained(
        &['㈜'],
        PrimaryClass::UnsupportedCharacterReview,
        Reason::UnsupportedCharacterReview
    )]
    #[case::unclassified(
        &[],
        PrimaryClass::UnclassifiedEncodingErrorReview,
        Reason::UnclassifiedEncodingErrorReview
    )]
    fn encoding_error_primary_requires_independent_pdf_evidence(
        #[case] singleton_unsupported_characters: &[char],
        #[case] expected_primary: PrimaryClass,
        #[case] expected_reason: Reason,
    ) {
        let encoded = EncodedCase {
            located: LocatedCase {
                shard: "synthetic.json".to_string(),
                index: 1,
                case: CorpusCase {
                    input: "입력".to_string(),
                    unicode: "expected".to_string(),
                },
            },
            actual: Err("encoding failed".to_string()),
            nfc_actual: None,
            nfkc_actual: None,
            singleton_unsupported_characters: singleton_unsupported_characters.to_vec(),
        };

        assert_eq!(
            classify(&encoded, &BTreeSet::new()),
            (expected_primary, expected_reason)
        );
    }

    #[test]
    fn whitespace_normalization_does_not_change_braille_cells() {
        assert_eq!(normalized_braille_whitespace("⠁ ⠃"), "⠁⠀⠃");
    }

    #[rstest::rstest]
    #[case::different_cells("⠁", "⠃", 0, "U+2801 ⠁ -> U+2803 ⠃")]
    #[case::expected_ended("", "⠃", 0, "<end> -> U+2803 ⠃")]
    #[case::actual_ended("⠁", "", 0, "U+2801 ⠁ -> <end>")]
    fn formats_first_difference_transition_key(
        #[case] expected: &str,
        #[case] actual: &str,
        #[case] index: usize,
        #[case] transition: &str,
    ) {
        assert_eq!(cell_transition_key(expected, actual, index), transition);
    }

    #[rstest::rstest]
    #[case::korean_suffix("값은 3.14이다.", vec!["3.14이다."])]
    #[case::roman_identifier("GPT-3.5보다", vec!["GPT-3.5보다"])]
    #[case::unit_and_punctuation("구간(1.0km), 종료", vec!["구간(1.0km),"])]
    #[case::multiple_points("주소 1.2.3 확인", vec!["1.2.3"])]
    #[case::period_not_between_digits("제3. 항목", vec![])]
    #[case::leading_decimal("값 .48", vec![])]
    fn detects_decimal_words(#[case] input: &str, #[case] expected: Vec<&str>) {
        let actual = decimal_word_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_decimal_word_in_current_korean_context_output() {
        let input = "수치는 34.3리터(L)이다.";
        let actual = braillify::encode_to_unicode(input).expect("decimal probe must encode");
        let ranges = korean_context_signature_ranges(input, &actual, &decimal_word_spans(input), 0);

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[rstest::rstest]
    #[case::energy("용량 13GWh 규모", vec![("13GWh", "GWh")])]
    #[case::distance("구간 0.73km", vec![("0.73km", "km")])]
    #[case::mass("필로폰 968g 등", vec![("968g", "g")])]
    #[case::ambiguous_variable("값 3x", vec![("3x", "x")])]
    #[case::letter_prefix("GPT3 모델", vec![])]
    #[case::alphanumeric_suffix("13GWh2", vec![])]
    fn detects_compact_numeric_ascii_suffixes(
        #[case] input: &str,
        #[case] expected: Vec<(&str, &str)>,
    ) {
        let actual = compact_numeric_ascii_suffix_spans(input)
            .into_iter()
            .map(|span| {
                (
                    &input[span.start_byte..span.end_byte],
                    compact_numeric_ascii_suffix(span, input),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_compact_numeric_ascii_suffix_and_entry_boundary_in_current_output() {
        let input = "용량은 13GWh 규모다.";
        let actual = braillify::encode_to_unicode(input).expect("compact suffix probe must encode");
        let spans = compact_numeric_ascii_suffix_spans(input);
        let ranges = korean_context_signature_ranges(input, &actual, &spans, 1);
        let signature = korean_context_signature(&input[spans[0].start_byte..spans[0].end_byte])
            .expect("compact suffix signature must encode");
        let signature_start_byte = actual
            .find(&signature)
            .expect("current output must contain compact suffix signature");
        let signature_start = actual[..signature_start_byte].chars().count();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start + 1, signature_start);
        assert_eq!(ranges[0].end, signature_start + signature.chars().count());
    }

    #[rstest::rstest]
    #[case::kilogram_parenthesis("상자(20kg)당", vec!["20kg)"])]
    #[case::metre_quote("길이는 3m”라고", vec!["3m”"])]
    #[case::ordinary_unit_boundary("무게는 3kg이다", vec![])]
    #[case::ambiguous_suffix("값은 3x)이다", vec![])]
    #[case::forced_slash_boundary("속도는 3m/시", vec![])]
    fn detects_rule69_ascii_units_before_terminator_skipping_symbols(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = rule69_ascii_unit_before_terminator_skipping_symbol_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn roman_indicator_moves_before_capital_word_indicator() {
        assert_eq!(roman_before_capital_order("⠠⠠⠴⠁⠃"), "⠴⠠⠠⠁⠃");
    }

    #[rstest::rstest]
    #[case::hca("HCA(Home Connectivity Alliance)", true)]
    #[case::embedded_in_korean("협회 HCA(Home Connectivity Alliance)는", true)]
    #[case::lowercase_headword("Hca(Home Connectivity Alliance)", false)]
    #[case::single_word_parenthetical("HCA(Alliance)", false)]
    #[case::unclosed_parenthetical("HCA(Home Connectivity Alliance", false)]
    #[case::operator_inside("AB(C + D)", false)]
    #[case::subscript_inside("AB(C D_1)", false)]
    #[case::nested_parenthetical("AB(C (D E))", false)]
    fn detects_uppercase_roman_headword_expansion(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(has_uppercase_roman_headword_expansion(input), expected);
    }

    #[rstest::rstest]
    #[case::person_label("학생 A(14)양", vec!["A(14)"])]
    #[case::standalone("A(1)", vec!["A(1)"])]
    #[case::multiple("A(11)과 B(15)", vec!["A(11)", "B(15)"])]
    #[case::lowercase("a(14)", vec![])]
    #[case::multi_capital("AB(14)", vec![])]
    #[case::empty_parenthetical("A()", vec![])]
    #[case::letter_argument("A(x)", vec![])]
    #[case::ascii_suffix("A(14)b", vec![])]
    fn detects_single_capital_parenthesized_digits(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = single_capital_parenthesized_digit_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::single_capital("미 F-35 전투기", vec!["F-35"])]
    #[case::multi_capital("육군 AH-64 헬기", vec!["AH-64"])]
    #[case::rule_35_shape("수능 D-100일", vec!["D-100"])]
    #[case::multiple("F-35와 AH-64", vec!["F-35", "AH-64"])]
    #[case::lowercase("x-1", vec![])]
    #[case::missing_digits("F-", vec![])]
    #[case::unicode_minus("F−35", vec![])]
    #[case::ascii_suffix("F-35A", vec![])]
    fn detects_uppercase_roman_hyphen_digits(#[case] input: &str, #[case] expected: Vec<&str>) {
        let actual = uppercase_roman_hyphen_digit_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_hyphen_digit_run_through_mixed_korean_routing() {
        let input = "한글 F-35 전투기";
        let spans = uppercase_roman_hyphen_digit_spans(input);
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = korean_context_signature_ranges(input, &actual, &spans, 1);

        assert_eq!(spans.len(), 1);
        assert_eq!(&input[spans[0].start_byte..spans[0].end_byte], "F-35");
        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
    }

    #[rstest::rstest]
    #[case::mixed_particle_before_expansion(
        "Matter와 HCA(Home Connectivity Alliance) 표준",
        vec!["HCA"]
    )]
    #[case::another_korean_suffix("Device는 ABC(Alpha Beta Company) 규격", vec!["ABC"])]
    #[case::korean_only_previous("기기와 HCA(Home Connectivity Alliance) 표준", vec![])]
    #[case::roman_only_previous("Matter HCA(Home Connectivity Alliance) 표준", vec![])]
    #[case::no_space_boundary("Matter와HCA(Home Connectivity Alliance)", vec![])]
    #[case::single_parenthetical_word("Matter와 HCA(Alliance)", vec![])]
    fn detects_mixed_roman_korean_boundary_before_headword_expansion(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = mixed_roman_korean_before_headword_expansion_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_later_headword_instead_of_earlier_roman_entry() {
        let input = "한글 Matter와 HCA(Home Connectivity Alliance) 표준";
        let spans = mixed_roman_korean_before_headword_expansion_spans(input);
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = current_engine_signature_ranges(input, &actual, &spans, 1);
        let first_roman_entry = actual
            .chars()
            .position(|cell| cell == '⠴')
            .expect("Matter must have an earlier Roman entry");

        assert_eq!(spans.len(), 1);
        assert_eq!(&input[spans[0].start_byte..spans[0].end_byte], "HCA");
        assert_eq!(ranges.len(), 1);
        assert!(first_roman_entry < ranges[0].start);
    }

    #[rstest::rstest]
    #[case::parenthesized_initialism("업무협약(MOU)을", vec!["MOU"])]
    #[case::standalone_word("SOUTH KOREA", vec!["SOUTH"])]
    #[case::multiple_runs("MOU와 YOUTH", vec!["MOU", "YOUTH"])]
    #[case::lowercase("Mou", vec![])]
    #[case::mixed_case("MoU", vec![])]
    #[case::no_ou("WHO", vec![])]
    #[case::digit_boundary("1MOU", vec![])]
    #[case::identifier_suffix("MOU2", vec![])]
    fn detects_allcaps_roman_runs_containing_ou(#[case] input: &str, #[case] expected: Vec<&str>) {
        let actual = allcaps_roman_runs_containing_ou(input)
            .into_iter()
            .map(|run| &input[run.start_byte..run.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::whole_shortform("가(WD) 나", vec!["WD"])]
    #[case::longer_prefixes("PDS LLM GDP", vec!["PDS", "LLM", "GDP"])]
    #[case::ueb_examples("ALT NEC LLC", vec!["ALT", "NEC", "LLC"])]
    #[case::noncolliding_controls("US KBS MCH", vec![])]
    #[case::alphanumeric_excluded("O4O Li2S V2X", vec![])]
    fn detects_allcaps_shortform_prefix_collisions(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = allcaps_shortform_prefix_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::mixed_identifiers("O4O Li2S V2X", vec!["O4O", "Li2S", "V2X"])]
    #[case::multiple_boundaries("A1B2C", vec!["A1B2C"])]
    #[case::lowercase_after_digit("240mg", vec![])]
    #[case::hyphenated_identifier("U-ENTER", vec![])]
    fn detects_uppercase_after_digit_in_roman_sequence(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = roman_uppercase_after_digit_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::uppercase_segments("U-ENTER CD-ROM", vec!["U-ENTER", "CD-ROM"])]
    #[case::digit_after_hyphen("F-35", vec![])]
    #[case::lowercase_after_hyphen("U-enter", vec![])]
    fn detects_uppercase_after_hyphen_in_roman_sequence(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = roman_uppercase_after_hyphen_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::pdf_complete_letters_sequence("CD-ROM", vec!["CD-ROM"])]
    #[case::single_capital_control("Around-U", vec![])]
    #[case::mixed_prefix_control("Ko-LLM", vec![])]
    #[case::digit_hyphen_control("F-35", vec![])]
    fn detects_only_pure_allcaps_hyphen_multi_allcaps_engine_boundary(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = pure_allcaps_hyphen_multi_allcaps_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::after_korean_word("연구단, A-STAR 방문", vec!["A-STAR"])]
    #[case::after_ascii_word("research A-STAR 방문", vec![])]
    #[case::without_whitespace("연구단,A-STAR 방문", vec![])]
    #[case::digit_hyphen_is_separate("연구단 F-35 방문", vec![])]
    #[case::numeric_leading_run("지표 2-CE 결과", vec![])]
    fn detects_hyphenated_roman_word_after_korean_word(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = roman_hyphenated_word_after_korean_word_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::mixed_case_headword("시스템 ccNC(connected car) 탑재", vec!["ccNC"])]
    #[case::allcaps_headword("줌 URL(ID : 3) 입력", vec!["URL"])]
    #[case::after_ascii_word("system URL(ID) 입력", vec![])]
    #[case::single_letter("시스템 A(x) 입력", vec![])]
    #[case::nested_parenthetical("시스템 URL(ID(x)) 입력", vec![])]
    fn detects_parenthetical_roman_headword_after_korean_word(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = roman_parenthetical_headword_after_korean_word_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::attached_structure("퀀텀닷(QD)-OLED 패널", vec!["(QD)-OLED"])]
    #[case::without_korean_prefix("(QD)-OLED 패널", vec![])]
    #[case::mixed_case_body("퀀텀닷(Qd)-OLED 패널", vec![])]
    #[case::single_cap_suffix("퀀텀닷(QD)-O 패널", vec![])]
    fn detects_korean_prefixed_parenthetical_hyphen_suffix(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = korean_prefixed_roman_parenthetical_hyphen_suffix_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::mixed_then_allcaps("Neo QLED는", vec!["QLED"])]
    #[case::allcaps_pair("DO DREAM)", vec!["DREAM"])]
    #[case::pdf_capital_passage("WELCOME TO KOREA", vec!["TO", "KOREA"])]
    #[case::punctuation_break("Neo. QLED는", vec![])]
    #[case::mixed_case_second("Neo Qled는", vec![])]
    #[case::korean_previous("한글 QLED는", vec![])]
    fn detects_consecutive_roman_uppercase_word_reentry(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = consecutive_roman_uppercase_word_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_current_reentry_before_consecutive_uppercase_word() {
        let input = "가 Neo QLED는";
        let spans = consecutive_roman_uppercase_word_spans(input);
        let actual = braillify::encode_to_unicode(input).expect("Roman reentry probe must encode");
        let ranges = roman_entry_signature_ranges(input, &actual, &spans, 0);
        let starts = ranges
            .iter()
            .map(|range| range.start)
            .collect::<BTreeSet<_>>();

        assert_eq!(spans.len(), 1);
        assert_eq!(starts.len(), 1);
        assert_eq!(actual.chars().nth(*starts.first().unwrap()), Some('⠴'));
    }

    #[rstest::rstest]
    #[case::shortform_prefix("가(WD) 나", allcaps_shortform_prefix_spans("가(WD) 나"))]
    #[case::numeric_continuation("가(Li2S) 나", roman_uppercase_after_digit_spans("가(Li2S) 나"))]
    #[case::hyphen_continuation(
        "가(U-ENTER) 나",
        roman_uppercase_after_hyphen_spans("가(U-ENTER) 나")
    )]
    fn locates_grade1_cohort_signature_in_korean_context(
        #[case] input: &str,
        #[case] spans: Vec<InputSpan>,
    ) {
        let actual = braillify::encode_to_unicode(input).expect("grade-1 probe must encode");
        let ranges = roman_entry_signature_ranges(input, &actual, &spans, 1);

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[rstest::rstest]
    #[case::hyphenated_word(
        "연구단, A-STAR 방문",
        roman_hyphenated_word_after_korean_word_spans("연구단, A-STAR 방문")
    )]
    #[case::parenthetical_headword(
        "시스템 ccNC(connected car) 탑재",
        roman_parenthetical_headword_after_korean_word_spans("시스템 ccNC(connected car) 탑재")
    )]
    #[case::attached_parenthetical_suffix(
        "퀀텀닷(QD)-OLED 패널",
        korean_prefixed_roman_parenthetical_hyphen_suffix_spans("퀀텀닷(QD)-OLED 패널")
    )]
    fn locates_roman_entry_residual_boundary(#[case] input: &str, #[case] spans: Vec<InputSpan>) {
        let actual = braillify::encode_to_unicode(input).expect("Roman entry probe must encode");
        let ranges = current_engine_input_entry_ranges(input, &actual, &spans, 2);

        assert_eq!(spans.len(), 1);
        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[test]
    fn locates_allcaps_ou_signature_in_complete_output() {
        let input = "업무협약(MOU)을 체결했다.";
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = allcaps_ou_actual_ranges(input, &actual);

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[rstest::rstest]
    #[case::standalone("새로운 DRX 브랜드", true)]
    #[case::inside_parentheses("엠디(MD), SNS", true)]
    #[case::expansion_headword("HCA(Home Connectivity Alliance)", false)]
    #[case::single_capital("점 A가 있다", false)]
    #[case::mixed_case("SmartThings Hub", false)]
    #[case::alphanumeric("O4O 시스템", false)]
    #[case::chemical_formula("PETCO2이다", false)]
    #[case::lowercase("web service", false)]
    fn detects_standalone_uppercase_roman_word(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(has_standalone_uppercase_roman_word(input), expected);
    }

    #[rstest::rstest]
    #[case::acronym("최고운영책임자(COO)", true)]
    #[case::organization("국가안전보장회의(NSC)를", true)]
    #[case::chemical_formula("일산화탄소(CO)는", true)]
    #[case::space_before_parenthesis("책임자 (COO)", false)]
    #[case::roman_prefix("HCA(COO)", false)]
    #[case::mixed_case("책임자(Ceo)", false)]
    #[case::digit_inside("규격(CO2)", false)]
    #[case::unclosed("책임자(COO", false)]
    fn detects_korean_prefixed_allcaps_parenthetical(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(has_korean_prefixed_allcaps_parenthetical(input), expected);
    }

    #[rstest::rstest]
    #[case::pdf_example("링컨(Lincoln)은", vec!["(Lincoln)"])]
    #[case::allcaps_annotation("엠디(MD),", vec!["(MD)"])]
    #[case::roman_suffix("폐쇄회로(CC)TV", vec!["(CC)"])]
    #[case::alphanumeric("표기(O4O)는", vec!["(O4O)"])]
    #[case::space_in_body("표기(Home Alliance)는", vec![])]
    #[case::operator_in_body("수식(x+1)은", vec![])]
    #[case::roman_prefix("HCA(Home)는", vec![])]
    #[case::space_before_open("표기 (MD)는", vec![])]
    #[case::unclosed("표기(MD", vec![])]
    fn detects_korean_prefixed_closed_roman_annotations(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = korean_prefixed_closed_roman_annotation_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn locates_rule_34_opening_after_the_real_korean_prefix() {
        let input = "앞말 링컨(Lincoln)은";
        let actual = braillify::encode_to_unicode(input).expect("rule-34 probe must encode");
        let ranges = korean_prefixed_annotation_opening_ranges(input, &actual);

        assert_eq!(ranges.len(), 1);
        assert_eq!(actual.chars().nth(ranges[0].start), Some('⠦'));
        assert_eq!(actual.chars().nth(ranges[0].start + 1), Some('⠄'));
    }

    #[test]
    fn classifies_only_the_rule_34_three_cell_reference_order_as_corpus_suspect() {
        let input = "링컨(Lincoln)은";
        let actual = braillify::encode_to_unicode(input).expect("rule-34 probe must encode");
        let opening = korean_prefixed_annotation_opening_ranges(input, &actual)
            .into_iter()
            .next()
            .expect("opening must be localized");
        let mut expected = actual.chars().collect::<Vec<_>>();
        expected.splice(opening.start..opening.start + 3, ['⠴', '⠐', '⠣']);
        let encoded = EncodedCase {
            located: LocatedCase {
                shard: "synthetic.json".to_string(),
                index: 1,
                case: CorpusCase {
                    input: input.to_string(),
                    unicode: expected.into_iter().collect(),
                },
            },
            actual: Ok(actual),
            nfc_actual: None,
            nfkc_actual: None,
            singleton_unsupported_characters: Vec::new(),
        };

        assert!(is_rule_34_reference_order_contradiction(&encoded));
        assert_eq!(
            classify(&encoded, &BTreeSet::new()),
            (
                PrimaryClass::CorpusSuspect,
                Reason::Rule34RomanIndicatorBeforeOpeningParenthesis
            )
        );
    }

    #[rstest::rstest]
    #[case::embedded_in_korean("AI·SW교육", true)]
    #[case::standalone("DRX·SNS", true)]
    #[case::lowercase("a·b", false)]
    #[case::single_letters("A·B", false)]
    #[case::korean("가·나", false)]
    #[case::numeric("3·1 운동", false)]
    #[case::space_separated("AI · SW", false)]
    #[case::alphanumeric_boundary("1AI·SW2", false)]
    fn detects_allcaps_roman_middle_dot_runs(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(has_allcaps_roman_middle_dot_runs(input), expected);
    }

    #[rstest::rstest]
    #[case::roman_korean("신작 PC·모바일", vec!["PC·모"])]
    #[case::roman_roman("AI·SW교육", vec!["AI·SW"])]
    #[case::mixed_case("기관(Fed·연준)", vec!["Fed·연"])]
    #[case::korean_only("온·오프라인", vec![])]
    #[case::numeric("3·1 운동", vec![])]
    #[case::spaced("AI · SW", vec![])]
    fn detects_roman_run_before_middle_dot_boundary(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        let actual = roman_run_before_middle_dot_boundary_spans(input)
            .into_iter()
            .map(|span| &input[span.start_byte..span.end_byte])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case::roman_korean("신작 PC·모바일")]
    #[case::roman_roman("AI·SW교육")]
    #[case::mixed_case("기관(Fed·연준)")]
    fn localizes_roman_middle_dot_boundary_in_complete_output(#[case] input: &str) {
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = korean_context_signature_ranges(
            input,
            &actual,
            &roman_run_before_middle_dot_boundary_spans(input),
            0,
        );

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[rstest::rstest]
    #[case::plus("양(+)극", vec!['+'])]
    #[case::hyphen_minus("음(-)극", vec!['-'])]
    #[case::unicode_minus("음(−)극", vec!['−'])]
    #[case::times("항(×)목", vec!['×'])]
    #[case::division("항(÷)목", vec!['÷'])]
    #[case::equals("항(=)목", vec!['='])]
    #[case::standalone("(+) 전극", vec![])]
    #[case::space_inside("양( + )극", vec![])]
    fn detects_inline_parenthesized_operators(
        #[case] input: &str,
        #[case] expected_operators: Vec<char>,
    ) {
        assert_eq!(
            inline_parenthesized_operators(input)
                .into_iter()
                .map(|candidate| candidate.operator)
                .collect::<Vec<_>>(),
            expected_operators
        );
    }

    #[test]
    fn locates_current_engine_parenthesized_operator_output() {
        let input = "양(+)극";
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = inline_parenthesized_operator_actual_ranges(input, &actual);

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[rstest::rstest]
    #[case::tight("△보성군", 1)]
    #[case::embedded("목록 △교과전형", 1)]
    #[case::spaced("△ 보성군", 0)]
    #[case::repeated_omission("△△ 종목", 0)]
    #[case::square_bullet("□2021", 0)]
    fn detects_tight_triangle_before_korean(#[case] input: &str, #[case] expected: usize) {
        assert_eq!(tight_triangle_positions(input).len(), expected);
    }

    #[rstest::rstest]
    #[case::leading("△보성군")]
    #[case::embedded("목록 △교과전형")]
    #[case::after_roman_context("MOU 협약 뒤 △항목")]
    fn locates_tight_triangle_and_first_korean_output(#[case] input: &str) {
        let actual = braillify::encode_to_unicode(input).expect("probe must encode");
        let ranges = tight_triangle_actual_ranges(input, &actual);

        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start < ranges[0].end);
        assert!(ranges[0].end <= actual.chars().count());
    }

    #[test]
    fn aggregates_headword_expansion_outcomes_without_reclassification() {
        let cases = [
            ("HCA(Home Connectivity Alliance)", "exact", "exact"),
            ("WHO(World Health Organization)", "expected", "actual"),
        ]
        .into_iter()
        .enumerate()
        .map(|(offset, (input, expected, actual))| {
            let located = LocatedCase {
                shard: "synthetic.json".to_string(),
                index: offset + 1,
                case: CorpusCase {
                    input: input.to_string(),
                    unicode: expected.to_string(),
                },
            };
            let encoded = EncodedCase {
                located: located.clone(),
                actual: Ok(actual.to_string()),
                nfc_actual: None,
                nfkc_actual: None,
                singleton_unsupported_characters: Vec::new(),
            };
            (located, encoded)
        })
        .collect::<Vec<_>>();
        let (located, encoded): (Vec<_>, Vec<_>) = cases.into_iter().unzip();

        let report = analyze(located, encoded, 5);
        let stats = report
            .pending_rule_review_clusters
            .get(UPPERCASE_ROMAN_HEADWORD_EXPANSION)
            .unwrap();

        assert_eq!((stats.candidates, stats.exact, stats.mismatch), (2, 1, 1));
        assert_eq!(stats.conflicting_reference_cases, 0);
        assert_eq!(stats.mismatch_primary_classes["pending_rule_review"], 1);
        assert_eq!(stats.samples["mismatch"][0].expected_excerpt, "expected");
        assert_eq!(stats.samples["mismatch"][0].actual_excerpt, "actual");
        assert_eq!(stats.samples["mismatch"][0].first_difference_cell, Some(0));
        assert_eq!(report.primary_classes["exact"], 1);
        assert_eq!(report.primary_classes["pending_rule_review"], 1);
    }
}
