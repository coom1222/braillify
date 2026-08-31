//! Reproducible NIKL Korean–Korean Braille Parallel Corpus 2025 v1.0 analysis.
//!
//! Run from the workspace root:
//! `cargo run --release -p braillify --example nikl_corpus_analyze`
//!
//! This is an offline evaluation tool. It deliberately deserializes only `input` and
//! `unicode`; the read-only competitor `world` field is neither loaded nor compared.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::thread;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum PrimaryClass {
    Exact,
    ImplementationDefect,
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
    EncodingError,
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

fn encode_cases(cases: &[LocatedCase], thread_count: usize) -> Vec<EncodedCase> {
    let chunk_size = cases.len().div_ceil(thread_count);
    let mut chunks = thread::scope(|scope| {
        cases
            .chunks(chunk_size.max(1))
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .cloned()
                        .map(|located| {
                            let actual = braillify::encode_to_unicode(&located.case.input);
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
        Err(_) => (PrimaryClass::ImplementationDefect, Reason::EncodingError),
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

fn is_delimiter_or_quote(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')' | '[' | ']' | '{' | '}' | '“' | '”' | '‘' | '’' | '"' | '\''
    )
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
    let first_diff = expected_chars
        .iter()
        .zip(&actual_chars)
        .position(|(left, right)| left != right)
        .unwrap_or(expected_chars.len().min(actual_chars.len()));
    let start = first_diff.saturating_sub(8);
    let expected_excerpt = expected_chars.iter().skip(start).take(24).collect();
    let actual_excerpt = actual_chars.iter().skip(start).take(24).collect();
    (expected_excerpt, actual_excerpt)
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
    } else if (0x2160..=0x217f).contains(&(ch as u32))
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
    let mut singleton_error_cache = BTreeMap::<char, bool>::new();
    let mut traits = BTreeMap::new();
    let mut shards = BTreeMap::<String, ShardStats>::new();
    let mut samples = BTreeMap::<String, Vec<Sample>>::new();
    let mut exact = 0usize;

    for item in &encoded {
        let (primary, reason) = classify(item, &conflicting);
        let primary_key = enum_key(&primary);
        let reason_key = enum_key(&reason);
        *primary_classes.entry(primary_key).or_insert(0) += 1;
        *reasons.entry(reason_key.clone()).or_insert(0) += 1;

        let shard = shards.entry(item.located.shard.clone()).or_default();
        shard.total += 1;
        if primary == PrimaryClass::Exact {
            exact += 1;
            shard.exact += 1;
            continue;
        }

        let input = &item.located.case.input;
        if primary == PrimaryClass::ImplementationDefect
            && let Err(error) = &item.actual
        {
            *encoding_error_messages.entry(error.clone()).or_insert(0) += 1;
            let unique_chars = input.chars().collect::<BTreeSet<_>>();
            let mut case_families = BTreeSet::new();
            for ch in unique_chars {
                let fails_alone = *singleton_error_cache
                    .entry(ch)
                    .or_insert_with(|| braillify::encode_to_unicode(&ch.to_string()).is_err());
                if fails_alone {
                    let key = format!("U+{:04X} {ch}", ch as u32);
                    let family = encoding_error_family(ch);
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
            }
            for family in case_families {
                *encoding_error_families
                    .entry(family.to_string())
                    .or_insert(0) += 1;
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
         The tool reads only `input` and `unicode`; it never loads or compares `world`.\n\n",
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
         (the rules 28/29 roman-indicator ordering signature) and actual encoding errors. \
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

    text.push_str("\n## Encoding-error diagnostics\n\n");
    text.push_str(
        "These are overlapping diagnostics for `implementation_defect` encoding errors, not additional primary classes. \
         A singleton error character is a character that also fails when encoded by itself.\n\n",
    );
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
    let config = Config::parse()?;
    let cases = load_cases()?;
    let encoded = encode_cases(&cases, config.threads);
    let report = analyze(cases, encoded, config.sample_limit);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize analysis JSON: {error}"))?;
    write_file(&config.json_path, &json)?;
    write_file(&config.report_path, &markdown(&report))?;
    println!(
        "NIKL corpus: {}/{} exact ({:.2}%), report={}, json={}",
        report.exact,
        report.total,
        report.exact_percent,
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

    #[rstest::rstest]
    #[case::compatibility_unit('㎏', "compatibility_unit_symbol")]
    #[case::roman_numeral('Ⅱ', "roman_numeral_presentation")]
    #[case::company_mark('㈜', "enclosed_organization_mark")]
    #[case::layout_symbol('▲', "punctuation_or_layout_symbol")]
    #[case::non_unit_square_log('㏒', "other_unsupported_symbol")]
    fn clusters_encoding_error_characters(#[case] input: char, #[case] expected_family: &str) {
        assert_eq!(encoding_error_family(input), expected_family);
    }

    #[test]
    fn whitespace_normalization_does_not_change_braille_cells() {
        assert_eq!(normalized_braille_whitespace("⠁ ⠃"), "⠁⠀⠃");
    }

    #[test]
    fn roman_indicator_moves_before_capital_word_indicator() {
        assert_eq!(roman_before_capital_order("⠠⠠⠴⠁⠃"), "⠴⠠⠠⠁⠃");
    }
}
