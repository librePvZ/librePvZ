/*
 * reanim-decode: decoder for PvZ reanim files.
 * Copyright (c) 2022  Ruifeng Xie
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Command line interface for `reanim-decode`.

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::{ArgEnum, Parser, Subcommand};
use fern::colors::Color::*;
use fern::colors::ColoredLevelConfig;
use log::LevelFilter;
use crate::reanim::Animation;
use crate::stream::Decode;
use crate::xml::Xml;

/// Entry of the command line interface.
#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Verbosity, for filtering diagnostics messages.
    #[clap(long, default_value_t = LevelFilter::Info)]
    pub verbose: LevelFilter,
    /// All available subcommands.
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Output format: JSON and YAML supported, guarded by crate features.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ArgEnum)]
pub enum Format {
    /// Internal encoding (Rust `{:#?}` debug pretty printing).
    Internal,
    /// XML format as is used in original PvZ game.
    Xml,
    /// JSON format. Guarded by crate feature `json` (enabled by default).
    #[cfg(feature = "json")]
    Json,
    /// YAML format. Guarded by crate feature `yaml` (enabled by default).
    #[cfg(feature = "yaml")]
    Yaml,
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Decode `.reanim.compiled` files.
    Decode {
        /// File name to open.
        file: PathBuf,
        /// Output format.
        #[clap(short, long, arg_enum, default_value_t = Format::Xml)]
        format: Format,
        /// Output file path.
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
}

const COLOURS: ColoredLevelConfig = ColoredLevelConfig {
    error: BrightRed,
    warn: BrightYellow,
    info: BrightCyan,
    debug: BrightBlue,
    trace: Cyan,
};

fn trim_crate_name(target: &str) -> &str {
    const CRATE_PREFIX: &str = "reanim_decode::";
    target.strip_prefix(CRATE_PREFIX).unwrap_or(target)
}

fn setup_logger(verbose: LevelFilter) {
    fern::Dispatch::new()
        .format(|out, message, record|
            out.finish(format_args!(
                "{}: {}: {}",
                trim_crate_name(record.target()),
                COLOURS.color(record.level()),
                message,
            )))
        .level(verbose)
        .chain(std::io::stderr())
        .apply().unwrap();
}

impl Cli {
    /// Start command line interface.
    pub fn run() -> Result<()> {
        let args = Cli::parse();
        setup_logger(args.verbose);
        match args.commands {
            Commands::Decode { file, format, output } => {
                // open input & decode
                let file = File::open(&file)
                    .with_context(|| format!("failed to read file {file:?}"))?;
                let mut file = BufReader::new(file);
                let anim = Animation::decode(&mut file)?;

                // output file (or stdout)
                let mut file_output;
                let stdout;
                let mut stdout_lock;
                let out: &mut dyn Write;
                if let Some(output) = output {
                    file_output = File::create(&output)
                        .with_context(|| format!("failed to open output file {output:?}"))?;
                    out = &mut file_output;
                } else {
                    stdout = std::io::stdout();
                    stdout_lock = stdout.lock();
                    out = &mut stdout_lock;
                }

                // write output
                match format {
                    Format::Internal => writeln!(out, "{anim:#?}"),
                    Format::Xml => write!(out, "{}", Xml(anim)),
                    #[cfg(feature = "json")]
                    Format::Json => writeln!(out, "{}", serde_json::to_string(&anim)?),
                    #[cfg(feature = "yaml")]
                    Format::Yaml => writeln!(out, "{}", serde_yaml::to_string(&anim)?),
                }?
            }
        }
        Ok(())
    }
}
