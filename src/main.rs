use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use explicit::config::Config;
use explicit::engine::{self, Options};
use explicit::output::{self, Format};

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
    },
    /// Watch files and re-check on change.
    Watch {
        #[command(flatten)]
        common: CommonArgs,
    },
    /// List all rules with their default severity.
    Rules,
    /// Write a starter explicit.toml.
    Init,
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
        Command::Check { common, fix } => {
            let config = load_config(&common)?;
            let files = engine::discover(&common.paths, &config)?;
            let opts = Options {
                remote: !common.no_remote,
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
            let opts = Options {
                remote: !common.no_remote,
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
        Command::Rules => {
            for r in explicit::rules::RULES {
                let sev = r.default.map_or("off", |s| s.as_str());
                println!("{:<32} {:<8} {}", r.id, sev, r.description);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Init => {
            let path = PathBuf::from(explicit::config::CONFIG_FILE);
            if path.exists() {
                return Err(format!("{} already exists", path.display()));
            }
            std::fs::write(&path, include_str!("../explicit.example.toml"))
                .map_err(|e| e.to_string())?;
            println!("Wrote {}", path.display());
            Ok(ExitCode::SUCCESS)
        }
    }
}
