use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};
use std::env;
use std::path::PathBuf;

use skel::Skeleton;
use skel::util::normalize_path;

#[derive(Parser)]
#[command(version, about)]
struct CliOptions {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Apply,
    Verify,
}

fn main() -> Result<()> {
    let cli = CliOptions::parse();
    let current_dir = env::current_dir().into_diagnostic()?;

    let mut config_path = match cli.config.as_deref() {
        Some(path) => PathBuf::from(path),
        None => current_dir.clone(),
    };

    if config_path.is_relative() {
        config_path = current_dir.join(config_path);
    }

    if config_path.is_dir() {
        config_path.push(".skeleton.kdl");
    }
    config_path = normalize_path(&current_dir, &config_path)?;

    let skeleton = Skeleton::from_config_file(config_path)?;
    println!("{:#?}", &skeleton);

    Ok(())
}
