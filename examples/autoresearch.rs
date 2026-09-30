//! Fixed public development benchmark. No network, subprocesses, or results cache.
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ops::Range;
use std::path::PathBuf;
use std::time::Instant;

use explicit::config::Config;
use explicit::diagnostic::Diagnostic;
use explicit::engine::{self, Options};
use serde::Deserialize;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const LANGUAGES: [&str; 7] = ["de", "en", "es", "fi", "fr", "pt", "sv"];
const WARM_RUNS: usize = 5;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    language: String,
    category: String,
    correct: String,
    incorrect: String,
    error: String,
}

struct Expected {
    language: String,
    category: String,
    range: Option<Range<usize>>,
}

struct Workload {
    config: Config,
    expected: BTreeMap<PathBuf, Expected>,
    bytes: usize,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Counts {
    labels: usize,
    tp: usize,
    fp: usize,
}

impl Counts {
    fn precision(&self) -> f64 {
        ratio(self.tp, self.tp + self.fp)
    }

    fn recall(&self) -> f64 {
        ratio(self.tp, self.labels)
    }

    fn f1(&self) -> f64 {
        ratio(2 * self.tp, self.labels + self.tp + self.fp)
    }
}

fn ratio(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { n as f64 / d as f64 }
}

fn family(rule: &str) -> Option<&str> {
    if rule == "spelling" {
        Some("spelling")
    } else if rule.starts_with("grammar/") {
        Some("grammar")
    } else if rule.starts_with("slop/") {
        Some("slop")
    } else {
        None
    }
}

fn overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < a.end && b.start < b.end && a.start < b.end && b.start < a.end
}

fn gettext_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn fixtures() -> Result<Vec<Pair>, Box<dyn Error>> {
    let mut pairs = Vec::new();
    for data in [
        include_str!("../eval/multilingual/controls.json"),
        include_str!("../eval/multilingual/de-fr.json"),
        include_str!("../eval/multilingual/es-pt.json"),
    ] {
        pairs.extend(serde_json::from_str::<Vec<Pair>>(data)?);
    }
    let mut unique = BTreeSet::new();
    let mut counts = BTreeMap::new();
    for pair in &pairs {
        if !LANGUAGES.contains(&pair.language.as_str())
            || !["spelling", "grammar", "slop"].contains(&pair.category.as_str())
            || pair.correct.is_empty()
            || pair.correct == pair.incorrect
            || pair.error.is_empty()
            || pair.incorrect.match_indices(&pair.error).count() != 1
            || pair.correct.contains('\n')
            || pair.incorrect.contains('\n')
            || !unique.insert((&pair.language, &pair.incorrect))
        {
            return Err(format!("invalid fixture: {} / {}", pair.language, pair.incorrect).into());
        }
        *counts
            .entry((pair.language.as_str(), pair.category.as_str()))
            .or_insert(0) += 1;
    }
    for language in LANGUAGES {
        let multiplier = if ["en", "fi", "sv"].contains(&language) {
            1
        } else {
            2
        };
        for (category, count) in [("spelling", 5), ("grammar", 4), ("slop", 1)] {
            if counts.get(&(language, category)) != Some(&(count * multiplier)) {
                return Err(format!("wrong fixture count: {language}/{category}").into());
            }
        }
    }
    Ok(pairs)
}

fn prepare(root: &std::path::Path, pairs: &[Pair]) -> Result<Workload, Box<dyn Error>> {
    let mut config_text = String::from(
        "[general]\ncache = false\nrespect_gitignore = false\nexclude = ['explicit.toml']\n\n[prose]\ndialect = 'american'\n\n[links]\nremote = false\noffline = true\ncache = false\ncheck_same_repo = false\n",
    );
    for language in LANGUAGES {
        config_text.push_str(&format!(
            "\n[[overrides]]\npaths = ['comments/{language}/*.rs']\nlanguage = '{language}'\n"
        ));
    }
    let config_path = root.join("explicit.toml");
    std::fs::write(&config_path, config_text)?;
    let config = Config::load(&config_path)?;
    let mut expected = BTreeMap::new();
    let mut bytes = 0;
    for (index, pair) in pairs.iter().enumerate() {
        for (variant, sentence) in [("clean", &pair.correct), ("error", &pair.incorrect)] {
            for surface in ["document", "comments", "marked", "gettext"] {
                let language = &pair.language;
                let (extension, prefix, text, suffix) = match surface {
                    "document" => (
                        "md",
                        format!("---\nlang: {language}\n---\n\n# 1\n\n"),
                        sentence.clone(),
                        "\n",
                    ),
                    "comments" => ("rs", "// ".into(), sentence.clone(), "\n"),
                    "marked" => (
                        "md",
                        format!(
                            "---\nlang: en\n---\n\n# 1\n\n<!-- explicit-lang {language} -->\n\n"
                        ),
                        sentence.clone(),
                        "\n\n<!-- explicit-lang end -->\n",
                    ),
                    "gettext" => (
                        "po",
                        format!(
                            "msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Language: {language}\\n\"\n\nmsgid \"File.\"\nmsgstr \""
                        ),
                        gettext_escape(sentence),
                        "\"\n",
                    ),
                    _ => unreachable!(),
                };
                let range = if variant == "error" {
                    let offset = sentence.find(&pair.error).ok_or("missing label")?;
                    let start = prefix.len()
                        + if surface == "gettext" {
                            gettext_escape(&sentence[..offset]).len()
                        } else {
                            offset
                        };
                    let length = if surface == "gettext" {
                        gettext_escape(&pair.error).len()
                    } else {
                        pair.error.len()
                    };
                    Some(start..start + length)
                } else {
                    None
                };
                let source = format!("{prefix}{text}{suffix}");
                bytes += source.len();
                let rel = PathBuf::from(format!(
                    "{surface}/{language}/{index:03}-{variant}.{extension}"
                ));
                let path = root.join(&rel);
                std::fs::create_dir_all(path.parent().ok_or("missing parent")?)?;
                std::fs::write(path, source)?;
                expected.insert(
                    rel,
                    Expected {
                        language: language.clone(),
                        category: pair.category.clone(),
                        range,
                    },
                );
            }
        }
    }
    Ok(Workload {
        config,
        expected,
        bytes,
    })
}

fn run(config: &Config, expected_files: usize) -> Result<Vec<Diagnostic>, Box<dyn Error>> {
    let files = engine::discover(std::slice::from_ref(&config.root), config)?;
    if files.len() != expected_files {
        return Err(format!("discovery mismatch: {} != {expected_files}", files.len()).into());
    }
    let workspace = engine::build_workspace(&files, config);
    if workspace.files.len() != expected_files {
        return Err("workspace dropped benchmark inputs".into());
    }
    Ok(engine::check(
        &workspace,
        &files,
        config,
        &Options::default(),
    ))
}

fn score(
    findings: &[Diagnostic],
    expected: &BTreeMap<PathBuf, Expected>,
) -> Result<BTreeMap<(String, String), Counts>, Box<dyn Error>> {
    let mut counts = BTreeMap::<(String, String), Counts>::new();
    for sample in expected.values() {
        let count = counts
            .entry((sample.language.clone(), sample.category.clone()))
            .or_default();
        count.labels += usize::from(sample.range.is_some());
    }
    let mut hit = BTreeSet::new();
    for finding in findings {
        let Some(category) = family(&finding.rule) else {
            continue;
        };
        let sample = expected
            .get(&finding.path)
            .ok_or("finding outside workload")?;
        let count = counts
            .entry((sample.language.clone(), category.to_string()))
            .or_default();
        if category == sample.category
            && sample
                .range
                .as_ref()
                .is_some_and(|r| overlap(&finding.range, r))
            && hit.insert(&finding.path)
        {
            count.tp += 1;
        } else {
            count.fp += 1;
        }
    }
    Ok(counts)
}

fn signature(
    findings: &[Diagnostic],
) -> impl Iterator<Item = (&std::path::Path, &str, usize, usize)> {
    findings.iter().map(|d| {
        (
            d.path.as_path(),
            d.rule.as_str(),
            d.range.start,
            d.range.end,
        )
    })
}

fn peak_rss_mb() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // getrusage initializes the platform-native structure; no subprocess or RSS polling.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let usage = unsafe { usage.assume_init() };
    let bytes = if cfg!(target_os = "macos") {
        usage.ru_maxrss as f64
    } else {
        usage.ru_maxrss as f64 * 1024.0
    };
    Ok(bytes / (1024.0 * 1024.0))
}

fn main() -> Result<(), Box<dyn Error>> {
    let pairs = fixtures()?;
    let root = tempfile::tempdir()?;
    let Workload {
        config,
        expected,
        bytes,
    } = prepare(root.path(), &pairs)?;
    let start = Instant::now();
    let cold = run(&config, expected.len())?;
    let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
    let counts = score(&cold, &expected)?;
    let mut durations = Vec::with_capacity(WARM_RUNS);
    for _ in 0..WARM_RUNS {
        let start = Instant::now();
        let warm = run(&config, expected.len())?;
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
        if signature(&cold).ne(signature(&warm)) {
            return Err("non-deterministic findings between repeated workloads".into());
        }
    }
    durations.sort_by(f64::total_cmp);
    let warm_ms = durations[WARM_RUNS / 2];
    let mut macro_error = 0.0;
    let mut macro_slop = 0.0;
    let mut new_quality = 0.0;
    let mut total = Counts::default();
    let mut total_slop = Counts::default();
    for language in LANGUAGES {
        let mut errors = Counts::default();
        for category in ["spelling", "grammar"] {
            let c = &counts[&(language.into(), category.into())];
            errors.labels += c.labels;
            errors.tp += c.tp;
            errors.fp += c.fp;
        }
        let slop = &counts[&(language.into(), "slop".into())];
        macro_error += errors.f1();
        macro_slop += slop.f1();
        let quality = 100.0 * (0.9 * errors.f1() + 0.1 * slop.f1());
        if ["de", "es", "fr", "pt"].contains(&language) {
            new_quality += quality;
        }
        println!("METRIC quality_{language}={quality:.6}");
        println!(
            "METRIC error_precision_{language}={:.6}",
            100.0 * errors.precision()
        );
        println!(
            "METRIC error_recall_{language}={:.6}",
            100.0 * errors.recall()
        );
        total.labels += errors.labels;
        total.tp += errors.tp;
        total.fp += errors.fp;
        total_slop.labels += slop.labels;
        total_slop.tp += slop.tp;
        total_slop.fp += slop.fp;
    }
    println!(
        "METRIC detection_score={:.6}",
        100.0 * (0.9 * macro_error + 0.1 * macro_slop) / LANGUAGES.len() as f64
    );
    println!("METRIC new_language_score={:.6}", new_quality / 4.0);
    println!("METRIC error_precision={:.6}", 100.0 * total.precision());
    println!("METRIC error_recall={:.6}", 100.0 * total.recall());
    println!(
        "METRIC slop_precision={:.6}",
        100.0 * total_slop.precision()
    );
    println!("METRIC slop_recall={:.6}", 100.0 * total_slop.recall());
    println!("METRIC false_positives={}", total.fp + total_slop.fp);
    println!("METRIC cold_ms={cold_ms:.6}");
    println!("METRIC warm_ms={warm_ms:.6}");
    println!(
        "METRIC throughput_mib_s={:.6}",
        bytes as f64 / (1024.0 * 1024.0) / (warm_ms / 1000.0)
    );
    println!("METRIC peak_rss_mb={:.6}", peak_rss_mb()?);
    println!("METRIC workload_files={}", expected.len());
    println!("METRIC workload_bytes={bytes}");
    println!("METRIC error_labels={}", total.labels);
    println!("METRIC slop_labels={}", total_slop.labels);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use explicit::diagnostic::Severity;

    fn diagnostic(path: &str, rule: &str, range: Range<usize>) -> Diagnostic {
        Diagnostic {
            path: path.into(),
            rule: rule.into(),
            severity: Severity::Warning,
            range,
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
            text: String::new(),
            message: String::new(),
            help: None,
            suggestions: Vec::new(),
            fix: None,
        }
    }

    #[test]
    fn scoring_penalizes_duplicates_wrong_categories_and_clean_text() {
        let expected = BTreeMap::from([
            (
                PathBuf::from("error.md"),
                Expected {
                    language: "fr".into(),
                    category: "grammar".into(),
                    range: Some(10..16),
                },
            ),
            (
                PathBuf::from("clean.md"),
                Expected {
                    language: "fr".into(),
                    category: "grammar".into(),
                    range: None,
                },
            ),
        ]);
        let findings = [
            diagnostic("error.md", "grammar/Agreement", 12..15),
            diagnostic("error.md", "grammar/Other", 10..16),
            diagnostic("error.md", "spelling", 10..16),
            diagnostic("clean.md", "grammar/Agreement", 10..16),
            diagnostic("error.md", "md/line-length", 0..20),
        ];
        let counts = score(&findings, &expected).unwrap();
        let grammar = &counts[&("fr".into(), "grammar".into())];
        assert_eq!(
            *grammar,
            Counts {
                labels: 1,
                tp: 1,
                fp: 2
            }
        );
        assert_eq!(grammar.f1(), 0.5);
        assert_eq!(counts[&("fr".into(), "spelling".into())].fp, 1);
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn scoring_does_not_credit_adjacent_or_zero_width_spans() {
        let expected = BTreeMap::from([(
            PathBuf::from("error.md"),
            Expected {
                language: "de".into(),
                category: "spelling".into(),
                range: Some(10..16),
            },
        )]);
        let findings = [
            diagnostic("error.md", "spelling", 0..10),
            diagnostic("error.md", "spelling", 16..20),
            diagnostic("error.md", "spelling", 12..12),
        ];
        let counts = score(&findings, &expected).unwrap();
        assert_eq!(
            counts[&("de".into(), "spelling".into())],
            Counts {
                labels: 1,
                tp: 0,
                fp: 3
            }
        );
        assert_eq!(Counts::default().f1(), 0.0);
    }
}
