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

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::{ArgEnum, Parser, Subcommand};
use fern::colors::{Color::*, ColoredLevelConfig};
use log::LevelFilter;
#[cfg(feature = "packed")]
use libre_pvz_resources::sprite as packed;
use crate::reanim::Animation;
use crate::xml::Xml;

/// Entry of the command line interface.
#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Verbosity, for filtering diagnostics messages.
    #[clap(long, arg_enum, global = true)]
    pub verbose: Option<Option<Verbosity>>,
    /// All available subcommands.
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Output format: JSON and YAML supported, guarded by crate features.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ArgEnum)]
#[allow(missing_docs)]
pub enum Verbosity { Off, Error, Warn, Info, Debug, Trace }

impl From<Verbosity> for LevelFilter {
    fn from(verb: Verbosity) -> Self {
        match verb {
            Verbosity::Off => LevelFilter::Off,
            Verbosity::Error => LevelFilter::Error,
            Verbosity::Warn => LevelFilter::Warn,
            Verbosity::Info => LevelFilter::Info,
            Verbosity::Debug => LevelFilter::Debug,
            Verbosity::Trace => LevelFilter::Trace,
        }
    }
}

/// Output format: JSON and YAML supported, guarded by crate features.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ArgEnum)]
pub enum Format {
    /// Internal encoding (Rust `{:#?}` debug pretty printing).
    Internal,
    /// Packed animation format, for use in librePvZ.
    #[cfg(feature = "packed")]
    Packed,
    /// Packed animation format, dump as YAML file.
    #[cfg(all(feature = "packed", feature = "json"))]
    PackedJson,
    /// Packed animation format, dump as JSON file.
    #[cfg(all(feature = "packed", feature = "yaml"))]
    PackedYaml,
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
        #[clap(short, long, arg_enum)]
        format: Option<Format>,
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
        setup_logger(match args.verbose {
            None => LevelFilter::Error, // no '--verbose', only errors
            Some(None) => LevelFilter::Info, // default '--verbose'
            Some(Some(verbose)) => verbose.into(), // explicit '--verbose'
        });
        match args.commands {
            Commands::Decode { file, format, output } => {
                // open input & decode
                let file = File::open(&file)
                    .with_context(|| format!("failed to read file {file:?}"))?;
                let mut file = BufReader::new(file);
                let anim = Animation::decompress_and_decode(&mut file)?;

                // infer output format
                let format = match format {
                    Some(format) => format,
                    None => match output.as_ref().and_then(|p| Some((
                        p.extension()?.to_str()?,
                        p.file_stem().and_then(OsStr::to_str),
                    ))) {
                        Some(("txt", _)) => Format::Internal,
                        #[cfg(feature = "packed")]
                        Some(("anim", _)) => Format::Packed,
                        Some(("reanim", _)) | Some(("xml", _)) => Format::Xml,
                        #[cfg(all(feature = "packed", feature = "json"))]
                        Some(("json", Some(stem)))
                        if stem.ends_with("packed") => Format::PackedJson,
                        #[cfg(feature = "json")]
                        Some(("json", _)) => Format::Json,
                        #[cfg(all(feature = "packed", feature = "yaml"))]
                        Some(("yaml", Some(stem)))
                        if stem.ends_with("packed") => Format::PackedYaml,
                        #[cfg(feature = "yaml")]
                        Some(("yaml", _)) => Format::Yaml,
                        _ => Format::Xml,
                    }
                };

                // output file (or stdout)
                if let Some(output) = output {
                    let context = || format!("failed to open output file {output:?}");
                    let file = File::create(&output).with_context(context)?;
                    encode(anim, format, file)?;
                } else {
                    encode(anim, format, std::io::stdout().lock())?;
                }
            }
        }
        Ok(())
    }
}

/// Encode the animation into required format.
pub fn encode(anim: Animation, format: Format, mut output: impl Write) -> anyhow::Result<()> {
    match format {
        Format::Internal => writeln!(output, "{anim:#?}")?,
        #[cfg(feature = "packed")]
        Format::Packed => {
            let anim = packed::Animation::from(anim);
            bincode::encode_into_std_write(anim, &mut output, bincode::config::standard())?;
        }
        #[cfg(all(feature = "packed", feature = "json"))]
        Format::PackedJson => {
            let anim = packed::Animation::from(anim);
            serde_json::to_writer_pretty(output, &anim)?;
        }
        #[cfg(all(feature = "packed", feature = "yaml"))]
        Format::PackedYaml => {
            let anim = packed::Animation::from(anim);
            serde_yaml::to_writer(output, &anim)?;
        }
        Format::Xml => write!(output, "{}", Xml(anim))?,
        #[cfg(feature = "json")]
        Format::Json => serde_json::to_writer_pretty(output, &anim)?,
        #[cfg(feature = "yaml")]
        Format::Yaml => serde_yaml::to_writer(output, &anim)?,
    }
    Ok(())
}
