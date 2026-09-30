use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use explicit::config::Config;
use explicit::engine::{self, Options};
use explicit::output::{self, Format};

// Harper allocates many small vectors per sentence; the system allocator is slow at that.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
#[command(
    name = "explicit",
    version,
    about = "Lint prose in Markdown files and code comments: structure, spelling, grammar, AI slop and links"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct CommonArgs {
    /// Files or directories to check.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    /// Path to `explicit.toml` (default: search upward from the first path).
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
    /// Do not check http(s) links over the network (cached results are still used).
    #[arg(long)]
    offline: bool,
    /// Skip http(s) links entirely.
    #[arg(long)]
    no_remote: bool,
    /// Check files named on the command line even when excluded (config, .gitignore, .explicitignore).
    #[arg(long)]
    no_exclude: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum RulesFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum SuggestFormat {
    Toml,
    Json,
}

#[derive(Subcommand)]
enum VocabCommand {
    /// Suggest `[[vocab]]` / `[[entity]]` entries for capitalized words the spell check flags.
    Suggest {
        /// Files or directories to scan.
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
        /// Path to `explicit.toml` (default: search upward from the first path).
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = SuggestFormat::Toml)]
        format: SuggestFormat,
        /// Only suggest words and names seen at least this many times.
        #[arg(long, default_value_t = 1)]
        min_count: usize,
    },
    /// List the configured `[[vocab]]` terms and `[[entity]]` names.
    List {
        /// Path to `explicit.toml` (default: search upward from the cwd).
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = RulesFormat::Text)]
        format: RulesFormat,
    },
}

#[derive(Subcommand)]
enum Command {
    /// Check files once and exit (0 = clean, 1 = problems, 2 = error).
    Check {
        #[command(flatten)]
        common: CommonArgs,
        /// Apply safe fixes.
        #[arg(long)]
        fix: bool,
        /// Re-check every file instead of reusing cached results.
        #[arg(long)]
        no_cache: bool,
        /// Cache directory (default: `.explicit_cache` in the project root).
        #[arg(long, value_name = "PATH")]
        cache_dir: Option<PathBuf>,
        /// Spelling/grammar backend, overriding `prose.engine` (experimental).
        #[arg(long, value_enum, hide = true)]
        engine: Option<explicit::config::Engine>,
    },
    /// Watch files and re-check on change.
    Watch {
        #[command(flatten)]
        common: CommonArgs,
    },
    /// List all rules with their default severity.
    Rules {
        /// List every concrete rule: each Harper rule as `grammar/<Name>` and each `[[style]]` rule.
        #[arg(long)]
        all: bool,
        #[arg(long, value_enum, default_value_t = RulesFormat::Text)]
        format: RulesFormat,
        /// Path to `explicit.toml` for `style/*` rules (default: search upward from the cwd).
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Project vocabulary: suggest entries from unknown names, or list the configured ones.
    Vocab {
        #[command(subcommand)]
        command: VocabCommand,
    },
    /// Write a starter explicit.toml in the current directory.
    Init {
        /// Write even if explicit.toml exists here or a config is found in a parent directory.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("explicit: {e}");
            ExitCode::from(2)
        }
    }
}

fn load_config(common: &CommonArgs) -> Result<Config, String> {
    let mut config = match &common.config {
        Some(p) => Config::load(p)?,
        None => Config::discover(
            common
                .paths
                .first()
                .map_or(std::path::Path::new("."), |p| p.as_path()),
        )?,
    };
    if common.offline {
        config.links.offline = true;
    }
    if common.no_remote {
        config.links.remote = false;
    }
    Ok(config)
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Command::Check {
            common,
            fix,
            no_cache,
            cache_dir,
            engine,
        } => {
            let mut config = load_config(&common)?;
            if let Some(e) = engine {
                e.check_available()?;
                config.prose.engine = e;
            }
            let files = engine::discover_with(&common.paths, &config, !common.no_exclude)?;
            let resolved_cache_dir = cache_dir.unwrap_or_else(|| {
                config.root.join(
                    config
                        .general
                        .cache_dir
                        .as_deref()
                        .unwrap_or(std::path::Path::new(explicit::cache::DEFAULT_DIR)),
                )
            });
            // `--no-cache` disables the results cache; the link cache still uses this dir.
            let cache_dir = (!no_cache && config.general.cache).then(|| resolved_cache_dir.clone());
            // Stale entries are pruned only when the whole root was checked.
            let prune_cache = common
                .paths
                .iter()
                .all(|p| p.canonicalize().is_ok_and(|p| p == config.root));
            let opts = Options {
                remote: !common.no_remote,
                cache_dir,
                link_cache_dir: Some(resolved_cache_dir),
                prune_cache,
            };
            let mut ws = engine::build_workspace(&files, &config);
            let mut diags = engine::check(&ws, &files, &config, &opts);
            if fix {
                let n =
                    explicit::fix::write_fixes(&config.root, &diags).map_err(|e| e.to_string())?;
                if n > 0 {
                    eprintln!("Applied {n} fixes.");
                    ws = engine::build_workspace(&files, &config);
                    diags = engine::check(&ws, &files, &config, &opts);
                }
            }
            let sources: HashMap<PathBuf, String> = ws
                .files
                .values()
                .map(|a| (a.file.rel.clone(), a.file.text.clone()))
                .collect();
            output::write(
                common.format,
                &diags,
                &sources,
                &config.root,
                &mut std::io::stdout().lock(),
            )
            .map_err(|e| e.to_string())?;
            let failed = diags.iter().any(|d| d.severity >= config.general.fail_on);
            Ok(if failed {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Watch { common } => {
            // Watch keeps results in memory; no disk cache.
            let opts = Options {
                remote: !common.no_remote,
                ..Options::default()
            };
            // CLI overrides apply on every config (re)load.
            explicit::watch::run(
                &common.paths,
                &|| load_config(&common),
                common.format,
                &opts,
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Rules {
            all,
            format,
            config,
        } => {
            let config = match &config {
                Some(p) => Config::load(p)?,
                None => Config::discover(std::path::Path::new("."))?,
            };
            let rows = explicit::rules::rule_rows(&config, all);
            let mut out = std::io::stdout().lock();
            let res = match format {
                RulesFormat::Json => serde_json::to_writer_pretty(&mut out, &rows)
                    .map_err(std::io::Error::from)
                    .and_then(|()| writeln!(out)),
                RulesFormat::Text => {
                    let width = rows.iter().map(|r| r.id.len()).max().unwrap_or(0).max(32);
                    rows.iter().try_for_each(|r| {
                        let sev = r.default.map_or("off", |s| s.as_str());
                        writeln!(out, "{:<width$} {:<8} {}", r.id, sev, r.description)
                    })
                }
            };
            // `explicit rules | head` closes the pipe early; that is not an error.
            match res {
                Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => Err(e.to_string()),
                _ => Ok(ExitCode::SUCCESS),
            }
        }
        Command::Vocab { command } => run_vocab(command),
        Command::Init { force } => {
            let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
            let path = cwd.join(explicit::config::CONFIG_FILE);
            if !force {
                if path.exists() {
                    return Err(format!(
                        "{} already exists (use --force to overwrite)",
                        path.display()
                    ));
                }
                if let Some(found) = explicit::config::find_config(&cwd) {
                    return Err(format!(
                        "{} already applies to this directory (use --force to write {} anyway)",
                        found.display(),
                        path.display()
                    ));
                }
            }
            std::fs::write(&path, include_str!("../explicit.example.toml"))
                .map_err(|e| format!("{}: {e}", path.display()))?;
            println!("Wrote {}", path.display());
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn run_vocab(command: VocabCommand) -> Result<ExitCode, String> {
    let mut out = std::io::stdout().lock();
    let res = match command {
        VocabCommand::Suggest {
            paths,
            config,
            format,
            min_count,
        } => {
            let config = match &config {
                Some(p) => Config::load(p)?,
                None => Config::discover(paths.first().map_or(std::path::Path::new("."), |p| p))?,
            };
            let files = engine::discover(&paths, &config)?;
            let sugs = explicit::vocab::suggest(&files, &config, min_count);
            match format {
                SuggestFormat::Json => serde_json::to_writer_pretty(&mut out, &sugs)
                    .map_err(std::io::Error::from)
                    .and_then(|()| writeln!(out)),
                SuggestFormat::Toml => {
                    write!(out, "{}", explicit::vocab::to_toml(&sugs, files.len()))
                }
            }
        }
        VocabCommand::List { config, format } => {
            let config = match &config {
                Some(p) => Config::load(p)?,
                None => Config::discover(std::path::Path::new("."))?,
            };
            let rows = explicit::vocab::list_rows(&config);
            match format {
                RulesFormat::Json => serde_json::to_writer_pretty(&mut out, &rows)
                    .map_err(std::io::Error::from)
                    .and_then(|()| writeln!(out)),
                RulesFormat::Text if rows.is_empty() => writeln!(
                    out,
                    "No [[vocab]] or [[entity]] entries; `explicit vocab suggest` proposes some."
                ),
                RulesFormat::Text => write!(out, "{}", explicit::vocab::list_table(&rows)),
            }
        }
    };
    match res {
        Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => Err(e.to_string()),
        _ => Ok(ExitCode::SUCCESS),
    }
}
