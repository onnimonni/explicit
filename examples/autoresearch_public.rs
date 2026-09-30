//! Frozen public, sampled-span benchmark; surrounding unannotated prose is unscored.
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Instant;

use explicit::config::{Config, Level};
use explicit::diagnostic::Diagnostic;
use explicit::engine::{self, Options};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const LANGUAGES: [&str; 7] = ["de", "en", "es", "fi", "fr", "pt", "sv"];
const CATEGORIES: [&str; 3] = ["spelling", "grammar", "slop"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Gold {
    version: u32,
    seed: String,
    method: String,
    sources: Vec<Source>,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    id: String,
    cohort: String,
    url: String,
    raw_url: String,
    blob_sha: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    id: String,
    source: String,
    language: String,
    category: String,
    start: usize,
    end: usize,
    text: String,
    label: Label,
    #[serde(deserialize_with = "required_correction")]
    correction: Option<String>,
}

// The field itself is mandatory, even though clean targets explicitly use null.
fn required_correction<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Label {
    Error,
    Clean,
}

struct Span {
    target: usize,
    range: Range<usize>,
    positive: bool,
}

struct Workload {
    config: Config,
    spans: BTreeMap<PathBuf, Vec<Span>>,
    bytes: usize,
}

#[derive(Default)]
struct Counts {
    labels: usize,
    tp: usize,
    fp: usize,
}

impl Counts {
    fn add(&mut self, other: &Self) {
        self.labels += other.labels;
        self.tp += other.tp;
        self.fp += other.fp;
    }

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

// Explicit zero_division=1: a clean-only category is perfect exactly when FP=0.
fn ratio(n: usize, d: usize) -> f64 {
    if d == 0 { 1.0 } else { n as f64 / d as f64 }
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

fn hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn immutable_urls(source: &Source) -> bool {
    let Some(url) = source.url.strip_prefix("https://github.com/") else {
        return false;
    };
    let Some((repo, file)) = url.split_once("/blob/") else {
        return false;
    };
    let Some((commit, path)) = file.split_once('/') else {
        return false;
    };
    let mut parts = repo.split('/');
    if parts.next().is_none_or(str::is_empty)
        || parts.next().is_none_or(str::is_empty)
        || parts.next().is_some()
        || !hex(commit, 40)
        || path.is_empty()
    {
        return false;
    }
    source.raw_url == format!("https://raw.githubusercontent.com/{repo}/{commit}/{path}")
}

fn load_gold() -> Result<Gold, Box<dyn Error>> {
    let gold: Gold = serde_json::from_str(include_str!("../eval/public/annotations.json"))?;
    if gold.version != 1
        || gold.seed.is_empty()
        || gold.method.is_empty()
        || gold.sources.len() != 47
        || gold.targets.is_empty()
    {
        return Err("invalid public annotation header".into());
    }
    let mut ids = BTreeSet::new();
    for source in &gold.sources {
        if source.id.is_empty()
            || !source
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !LANGUAGES.contains(&source.cohort.as_str())
            || !source.id.starts_with(&format!("{}-", source.cohort))
            || !immutable_urls(source)
            || !hex(&source.blob_sha, 40)
            || !hex(&source.sha256, 64)
            || !ids.insert(source.id.as_str())
        {
            return Err(format!("invalid public source metadata: {}", source.id).into());
        }
    }
    let mut target_ids = BTreeSet::new();
    for target in &gold.targets {
        if target.id.is_empty()
            || !target_ids.insert(target.id.as_str())
            || !ids.contains(target.source.as_str())
            || !LANGUAGES.contains(&target.language.as_str())
            || !CATEGORIES.contains(&target.category.as_str())
            || target.start >= target.end
            || target.text.is_empty()
            || match target.label {
                Label::Error => target
                    .correction
                    .as_ref()
                    .is_none_or(|c| c.is_empty() || c == &target.text),
                Label::Clean => target.correction.is_some(),
            }
        {
            return Err(format!("invalid public target metadata: {}", target.id).into());
        }
    }
    for (i, a) in gold.targets.iter().enumerate() {
        for b in &gold.targets[i + 1..] {
            if a.source == b.source && overlap(&(a.start..a.end), &(b.start..b.end)) {
                if a.category == b.category && (a.label != b.label || a.language != b.language) {
                    return Err(format!("contradictory public targets: {} / {}", a.id, b.id).into());
                }
                if a.label == Label::Error && b.label == Label::Error {
                    return Err(
                        format!("overlapping public replacements: {} / {}", a.id, b.id).into(),
                    );
                }
            }
        }
    }
    Ok(gold)
}

fn config(root: &Path) -> Config {
    let mut config = Config {
        root: root.to_path_buf(),
        ..Config::default()
    };
    config.general.cache = false;
    config.general.respect_gitignore = false;
    config.prose.dialect = "american".into();
    config.links.remote = false;
    config.links.offline = true;
    config.links.cache = false;
    config.links.check_same_repo = false;
    for pattern in [
        "md/*",
        "docs/*",
        "codeblock/*",
        "diagram/*",
        "prose/*",
        "links/*",
    ] {
        config.rules.insert(pattern.into(), Level::Off);
    }
    config
}

fn prepare(
    gold: &Gold,
    corpus: &Path,
    original_root: &Path,
    corrected_root: &Path,
) -> Result<(Workload, Workload), Box<dyn Error>> {
    let corpus = corpus.canonicalize()?;
    if !corpus.is_dir() {
        return Err("EXPLICIT_PUBLIC_CORPUS must be a directory".into());
    }
    let original_root = original_root.canonicalize()?;
    let corrected_root = corrected_root.canonicalize()?;
    let mut original = Workload {
        config: config(&original_root),
        spans: BTreeMap::new(),
        bytes: 0,
    };
    let mut corrected = Workload {
        config: config(&corrected_root),
        spans: BTreeMap::new(),
        bytes: 0,
    };
    for source in &gold.sources {
        let rel = PathBuf::from(format!("{}.md", source.id));
        let input = corpus.join(&rel).canonicalize()?;
        if !input.starts_with(&corpus) || !input.is_file() {
            return Err(format!("public source escapes corpus: {}", source.id).into());
        }
        let text = std::fs::read_to_string(&input)?;
        if format!("{:x}", Sha256::digest(text.as_bytes())) != source.sha256 {
            return Err(format!("public source SHA256 mismatch: {}", source.id).into());
        }
        let mut targets: Vec<_> = gold
            .targets
            .iter()
            .enumerate()
            .filter(|(_, t)| t.source == source.id)
            .collect();
        targets.sort_by_key(|(i, t)| (t.start, t.end, *i));
        let mut original_spans = Vec::with_capacity(targets.len());
        for &(index, target) in &targets {
            if text.get(target.start..target.end) != Some(target.text.as_str()) {
                return Err(format!("public target byte/text mismatch: {}", target.id).into());
            }
            original_spans.push(Span {
                target: index,
                range: target.start..target.end,
                positive: target.label == Label::Error,
            });
        }
        let mut fixed = String::with_capacity(text.len());
        let mut corrected_spans = Vec::new();
        let mut cursor = 0;
        for &(index, target) in &targets {
            if target.label != Label::Error {
                continue;
            }
            fixed.push_str(&text[cursor..target.start]);
            let start = fixed.len();
            let replacement = target.correction.as_deref().ok_or("missing correction")?;
            fixed.push_str(replacement);
            corrected_spans.push(Span {
                target: index,
                range: start..fixed.len(),
                positive: false,
            });
            cursor = target.end;
        }
        fixed.push_str(&text[cursor..]);
        std::fs::write(original_root.join(&rel), &text)?;
        std::fs::write(corrected_root.join(&rel), &fixed)?;
        original.bytes += text.len();
        corrected.bytes += fixed.len();
        original.spans.insert(rel.clone(), original_spans);
        corrected.spans.insert(rel, corrected_spans);
    }
    Ok((original, corrected))
}

fn run(workload: &Workload) -> Result<Vec<Diagnostic>, Box<dyn Error>> {
    let files = engine::discover(
        std::slice::from_ref(&workload.config.root),
        &workload.config,
    )?;
    if files.len() != workload.spans.len() {
        return Err(format!(
            "public discovery mismatch: {} != {}",
            files.len(),
            workload.spans.len()
        )
        .into());
    }
    let workspace = engine::build_workspace(&files, &workload.config);
    if workspace.files.len() != workload.spans.len() {
        return Err("workspace dropped public benchmark inputs".into());
    }
    Ok(engine::check(
        &workspace,
        &files,
        &workload.config,
        &Options::default(),
    ))
}

type Scores = BTreeMap<(&'static str, &'static str), Counts>;

fn score(
    gold: &Gold,
    workload: &Workload,
    findings: &[Diagnostic],
    counts: &mut Scores,
) -> Result<usize, Box<dyn Error>> {
    let key = |target: &Target| {
        let language = LANGUAGES
            .iter()
            .copied()
            .find(|l| *l == target.language)
            .expect("validated language");
        let category = CATEGORIES
            .iter()
            .copied()
            .find(|c| *c == target.category)
            .expect("validated category");
        (language, category)
    };
    for spans in workload.spans.values() {
        for span in spans {
            counts
                .get_mut(&key(&gold.targets[span.target]))
                .ok_or("missing score bucket")?
                .labels += usize::from(span.positive);
        }
    }
    let mut hit = BTreeSet::new();
    let mut ignored = 0;
    for finding in findings {
        let spans = workload
            .spans
            .get(&finding.path)
            .ok_or("finding outside public workload")?;
        let Some(category) = family(&finding.rule) else {
            ignored += 1;
            continue;
        };
        let matches = |span: &&Span| {
            gold.targets[span.target].category == category && overlap(&finding.range, &span.range)
        };
        let matched = spans
            .iter()
            .filter(matches)
            .find(|span| span.positive && !hit.contains(&span.target))
            .or_else(|| spans.iter().find(matches));
        let Some(span) = matched else {
            ignored += 1;
            continue;
        };
        let count = counts
            .get_mut(&key(&gold.targets[span.target]))
            .ok_or("missing score bucket")?;
        if span.positive && hit.insert(span.target) {
            count.tp += 1;
        } else {
            count.fp += 1;
        }
    }
    Ok(ignored)
}

fn peak_rss_mb() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // getrusage initializes the native structure; no subprocess or polling.
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
    let gold = load_gold()?;
    let corpus = PathBuf::from(std::env::var_os("EXPLICIT_PUBLIC_CORPUS").ok_or(
        "EXPLICIT_PUBLIC_CORPUS is required; run scripts/prepare-public-benchmark.sh first",
    )?);
    let original_root = tempfile::tempdir()?;
    let corrected_root = tempfile::tempdir()?;
    let (original, corrected) =
        prepare(&gold, &corpus, original_root.path(), corrected_root.path())?;
    let start = Instant::now();
    let original_findings = run(&original)?;
    let corrected_findings = run(&corrected)?;
    let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut counts: Scores = LANGUAGES
        .iter()
        .flat_map(|&language| {
            CATEGORIES
                .iter()
                .map(move |&category| ((language, category), Counts::default()))
        })
        .collect();
    let ignored = score(&gold, &original, &original_findings, &mut counts)?
        + score(&gold, &corrected, &corrected_findings, &mut counts)?;
    let mut total_error = Counts::default();
    let mut total_slop = Counts::default();
    let mut macro_quality = 0.0;
    for language in LANGUAGES {
        let mut errors = Counts::default();
        for category in CATEGORIES {
            let count = &counts[&(language, category)];
            println!("METRIC public_tp_{language}_{category}={}", count.tp);
            println!("METRIC public_fp_{language}_{category}={}", count.fp);
            println!(
                "METRIC public_labels_{language}_{category}={}",
                count.labels
            );
            if category != "slop" {
                errors.add(count);
            }
        }
        let slop = &counts[&(language, "slop")];
        let quality = 100.0 * (0.9 * errors.f1() + 0.1 * slop.f1());
        macro_quality += quality;
        println!("METRIC public_quality_{language}={quality:.6}");
        println!("METRIC public_tp_{language}_error={}", errors.tp);
        println!("METRIC public_fp_{language}_error={}", errors.fp);
        println!("METRIC public_labels_{language}_error={}", errors.labels);
        total_error.add(&errors);
        total_slop.add(slop);
    }
    println!(
        "METRIC public_detection_score={:.6}",
        macro_quality / LANGUAGES.len() as f64
    );
    println!(
        "METRIC public_error_precision={:.6}",
        100.0 * total_error.precision()
    );
    println!(
        "METRIC public_error_recall={:.6}",
        100.0 * total_error.recall()
    );
    println!(
        "METRIC public_slop_precision={:.6}",
        100.0 * total_slop.precision()
    );
    println!(
        "METRIC public_slop_recall={:.6}",
        100.0 * total_slop.recall()
    );
    println!(
        "METRIC public_false_positives={}",
        total_error.fp + total_slop.fp
    );
    println!("METRIC public_cold_ms={cold_ms:.6}");
    println!("METRIC public_peak_rss_mb={:.6}", peak_rss_mb()?);
    println!("METRIC public_error_labels={}", total_error.labels);
    println!("METRIC public_slop_labels={}", total_slop.labels);
    println!(
        "METRIC public_workload_files={}",
        original.spans.len() + corrected.spans.len()
    );
    println!(
        "METRIC public_workload_bytes={}",
        original.bytes + corrected.bytes
    );
    println!("METRIC public_ignored_findings={ignored}");
    let positives = gold
        .targets
        .iter()
        .filter(|t| t.label == Label::Error)
        .count();
    println!(
        "ASI public_gold_version={} sources={} targets={} errors={} clean={} corrected_negatives={}",
        gold.version,
        gold.sources.len(),
        gold.targets.len(),
        positives,
        gold.targets.len() - positives,
        positives
    );
    println!(
        "ASI public_scope=annotated_byte_overlap_same_family original_all_targets corrected_former_errors_only unannotated_findings_ignored zero_division=1 macro_languages=de,en,es,fi,fr,pt,sv error=spelling+grammar weights=0.9error+0.1slop"
    );
    Ok(())
}
