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

#[derive(Debug, Serialize)]
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
const ALLCAPS_ROMAN_MIDDLE_DOT_RUNS: &str =
    "multi_character_allcaps_roman_runs_joined_by_middle_dot";
const KOREAN_INLINE_PARENTHESIZED_OPERATOR: &str =
    "korean_inline_parenthesized_single_arithmetic_operator";
const TIGHT_TRIANGLE_BEFORE_KOREAN: &str = "tight_triangle_mark_immediately_before_korean";
const ALLCAPS_ROMAN_RUN_CONTAINING_OU: &str = "allcaps_roman_run_containing_ou";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AllcapsRomanRun {
    start_byte: usize,
    end_byte: usize,
}

/// Finds maximal all-caps ASCII runs containing the adjacent letters `OU`.
///
/// This is an input gate for a pronunciation-sensitive UEB diagnostic, not a
/// claim that the run is an initialism. Alphanumeric outer boundaries exclude
/// fragments of identifiers while retaining parenthesized and standalone runs.
fn allcaps_roman_runs_containing_ou(input: &str) -> Vec<AllcapsRomanRun> {
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
            runs.push(AllcapsRomanRun {
                start_byte,
                end_byte,
            });
        }
    }
    runs
}

/// Locates each detected run in the full current-engine output by searching
/// for that run's independently encoded signature. This uses neither the
/// corpus reference nor a hard-coded braille value.
fn allcaps_ou_actual_ranges(input: &str, actual: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = BTreeSet::new();
    for candidate in allcaps_roman_runs_containing_ou(input) {
        let run = &input[candidate.start_byte..candidate.end_byte];
        let Ok(signature) = braillify::encode_to_unicode(run) else {
            continue;
        };
        let signature_cells = signature.chars().count();
        for (start_byte, _) in actual.match_indices(&signature) {
            let start = actual[..start_byte].chars().count();
            ranges.insert((start, start + signature_cells));
        }
    }
    ranges.into_iter().map(|(start, end)| start..end).collect()
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

/// Input-only candidate gate for acronym expansions such as
/// `HCA(Home Connectivity Alliance)`.
///
/// This is deliberately an analyzer diagnostic, not an engine rule. Requiring
/// only ASCII letters and spaces inside the closed parenthesis also excludes
/// visible operators, subscript/superscript notation, and nested parentheses.
fn has_uppercase_roman_headword_expansion(input: &str) -> bool {
    let bytes = input.as_bytes();
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
            return true;
        }
    }
    false
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

fn tight_triangle_positions(input: &str) -> Vec<usize> {
    input
        .match_indices('△')
        .filter_map(|(byte, mark)| {
            input[byte + mark.len()..]
                .chars()
                .next()
                .is_some_and(is_korean_script)
                .then_some(byte)
        })
        .collect()
}

/// Current-engine ranges for `△한글`, including the first Korean cell after
/// the mark. A missing reference space therefore differs inside this range,
/// while unrelated earlier sentence differences do not count as causal.
fn tight_triangle_actual_ranges(input: &str, actual: &str) -> Vec<std::ops::Range<usize>> {
    let actual_cells = actual.chars().collect::<Vec<_>>();
    let marker = braillify::encode_to_unicode("△")
        .expect("triangle probe must encode")
        .chars()
        .collect::<Vec<_>>();
    tight_triangle_positions(input)
        .into_iter()
        .filter_map(|byte| {
            let start = braillify::encode_to_unicode(&input[..byte])
                .ok()?
                .chars()
                .count();
            let marker_end = start.checked_add(marker.len())?;
            let range_end = marker_end.checked_add(1)?;
            (actual_cells.get(start..marker_end) == Some(marker.as_slice())
                && actual_cells.get(marker_end).is_some())
            .then_some(start..range_end)
        })
        .collect()
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
    primary: PrimaryClass,
    primary_key: &str,
    reason_key: &str,
    sample_limit: usize,
    first_difference_in_output_signature: Option<bool>,
) {
    stats.candidates += 1;
    let outcome = if primary == PrimaryClass::Exact {
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
        if primary == PrimaryClass::CorpusSuspect {
            stats.conflicting_reference_cases += 1;
        }
        *stats
            .mismatch_primary_classes
            .entry(primary_key.to_string())
            .or_insert(0) += 1;
        "mismatch"
    };
    let bucket = stats.samples.entry(outcome.to_string()).or_default();
    if bucket.len() >= sample_limit
        || bucket
            .iter()
            .any(|sample| sample.shard == item.located.shard)
    {
        return;
    }

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
    bucket.push(PendingRuleReviewClusterSample {
        shard: item.located.shard.clone(),
        index: item.located.index,
        input: item.located.case.input.clone(),
        expected_excerpt,
        actual_excerpt,
        first_difference_cell,
        error,
        primary_class: primary_key.to_string(),
        reason: reason_key.to_string(),
    });
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
            ALLCAPS_ROMAN_RUN_CONTAINING_OU.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            ALLCAPS_ROMAN_MIDDLE_DOT_RUNS.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL.to_string(),
            PendingRuleReviewClusterStats::default(),
        ),
        (
            KOREAN_INLINE_PARENTHESIZED_OPERATOR.to_string(),
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
    ]);
    let mut pending_first_difference_cell_transitions = BTreeMap::new();
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
        }

        for (cluster, present, localized_first_difference) in [
            (
                ALLCAPS_ROMAN_RUN_CONTAINING_OU,
                !allcaps_roman_runs_containing_ou(&item.located.case.input).is_empty(),
                Some(first_difference_in_allcaps_ou_run(item)),
            ),
            (
                ALLCAPS_ROMAN_MIDDLE_DOT_RUNS,
                has_allcaps_roman_middle_dot_runs(&item.located.case.input),
                None,
            ),
            (
                KOREAN_PREFIXED_ALLCAPS_PARENTHETICAL,
                has_korean_prefixed_allcaps_parenthetical(&item.located.case.input),
                None,
            ),
            (
                KOREAN_INLINE_PARENTHESIZED_OPERATOR,
                !inline_parenthesized_operators(&item.located.case.input).is_empty(),
                Some(first_difference_in_inline_parenthesized_operator(item)),
            ),
            (
                STANDALONE_UPPERCASE_ROMAN_WORD,
                has_standalone_uppercase_roman_word(&item.located.case.input),
                None,
            ),
            (
                TIGHT_TRIANGLE_BEFORE_KOREAN,
                !tight_triangle_positions(&item.located.case.input).is_empty(),
                Some(first_difference_in_tight_triangle(item)),
            ),
            (
                UPPERCASE_ROMAN_HEADWORD_EXPANSION,
                has_uppercase_roman_headword_expansion(&item.located.case.input),
                None,
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
                primary,
                &primary_key,
                &reason_key,
                sample_limit,
                localized_first_difference,
            );
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

    text.push_str("\n## Cross-cutting input-only structural cohorts\n\n");
    text.push_str(
        "These are cross-cutting input-only structural cohorts, not new primary classes and not \
         engine routing rules. Candidate selection never changes a case's existing primary \
         class. Only cohort members already classified as `pending_rule_review` form a pending \
         subcluster; exact and other-primary members are controls that retain their existing \
         outcomes. The \
         `uppercase_roman_headword_closed_multiword_parenthetical` gate requires a two-or-more \
         character uppercase ASCII headword immediately followed by a closed parenthesis whose \
         contents are two or more ASCII Roman words separated only by spaces. Because the \
         contents admit only letters and spaces, visible operators, subscript/superscript \
         notation, and nested parentheses are excluded deterministically. The \
         `standalone_multi_character_uppercase_roman_word` gate finds maximal ASCII-letter runs \
         of two or more capitals with non-alphanumeric boundaries; a run immediately followed \
         by `(` is excluded so the HCA-style headword itself is not counted by both gates. The \
         `korean_prefixed_closed_allcaps_parenthetical` gate requires an immediately preceding \
         Korean character and a closed body of two or more uppercase ASCII letters. It \
         intentionally contains both acronym annotations (`책임자(COO)`) and scientific \
         formulae (`일산화탄소(CO)`) so their semantic collision remains measurable. The \
         `multi_character_allcaps_roman_runs_joined_by_middle_dot` gate requires two maximal \
         ASCII-letter runs of at least two capitals joined directly by U+00B7, with \
         non-alphanumeric outer boundaries. It records shapes such as `AI·SW` without assigning \
         prose, mathematics, or science semantics. The \
         `korean_inline_parenthesized_single_arithmetic_operator` gate requires an immediate \
         `Korean(` + one rule-45 arithmetic operator + `)Korean` span. Unlike broad coexistence \
         traits, it also locates the current engine's emitted structure and counts a mismatch as \
         signature-local only when the sentence's first differing cell falls inside that output \
         range. The `allcaps_roman_run_containing_ou` gate finds maximal, alphanumeric-delimited \
         uppercase ASCII runs containing adjacent `OU`. It locates the independently encoded run \
         signature in the complete current output and counts only first differences inside that \
         signature as localized. The `tight_triangle_mark_immediately_before_korean` gate requires literal \
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
             cohort does not reclassify them.\n\n",
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
    text.push_str(
        "\nThis shape is not an engine implementation premise. The 2024 PDF's math rule 6 \
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
         bullet. Localized mismatches are therefore corpus/layout review evidence only.\n\n\
         Corpus contradictions remain a separate gate: identical inputs with conflicting \
         references are classified as `corpus_suspect` before these cohorts are recorded and \
         would appear explicitly in each mismatch primary-class distribution. Their absence does \
         not prove a reference correct; it only means that this deterministic contradiction test \
         did not fire.\n",
    );
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
        text.push_str(&format!(
            " At this implementation checkpoint, the strict cohort moved from 0 to {} exact \
             cases; the corpus-wide total moved from 65,491 to {} ({:+} exact) because the same \
             PDF-backed spacing rule also applies outside the stricter Korean-boundary audit \
             gate. The complete standard suite remains 5,141/5,141.\n",
            stats.exact,
            report.exact,
            report.exact as isize - 65_491
        ));
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
         대통령이다.` Rule 54 says that text immediately after an opening bracket and \
         immediately before a closing bracket is attached. Together these establish the \
         Korean-prefix + closed-Roman-annotation context independently of corpus expected \
         values. A following comma or period is outside the already closed annotation and \
         must not cause its Roman contents to be rerouted as mathematics.\n\n\
         The implementation gate exists only inside `split_mixed_math_word`, after the prefix \
         has been proved entirely Korean. It accepts a fully closed parenthesized Roman word \
         (including ASCII digits such as `O4O`) plus ordinary trailing prose punctuation. \
         The global math detector is byte-for-byte unchanged; regression tests preserve its \
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
        "\nEngine changes must add a row only after both the 5,141-case standard suite and \
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

    #[test]
    fn locates_tight_triangle_and_first_korean_output() {
        let input = "△보성군";
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
