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
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use anyhow::Context;
use clap::{ArgEnum, Parser, Subcommand};
use fern::colors::{Color::*, ColoredLevelConfig};
use log::LevelFilter;
use serde::{Serialize, Serializer};
use libre_pvz_resources::animation as packed;
use crate::reanim::Animation;
use crate::xml::Xml;

/// Optionally packed animations.
pub enum MaybePacked {
    /// Plain format, structurally equivalent to reanim XML.
    Plain(Animation),
    /// Packed format, compact and structural.
    Packed(packed::AnimDesc),
}

impl Debug for MaybePacked {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybePacked::Plain(anim) => anim.fmt(f),
            MaybePacked::Packed(anim) => anim.fmt(f),
        }
    }
}

impl Serialize for MaybePacked {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MaybePacked::Plain(anim) => anim.serialize(serializer),
            MaybePacked::Packed(anim) => anim.serialize(serializer),
        }
    }
}

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
    /// Internal encoding (Rust `{:#?}` debug pretty printing)
    Internal,
    /// Binary encoding using `bincode`. Only support packed format.
    Bin,
    /// XML format as is used in original PvZ game.
    Xml,
    /// JSON format. Guarded by crate feature `json` (enabled by default).
    Json,
    /// YAML format. Guarded by crate feature `yaml` (enabled by default).
    Yaml,
}

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Format::Internal => "internal",
            Format::Bin => "bin",
            Format::Xml => "xml",
            Format::Json => "json",
            Format::Yaml => "yaml",
        })
    }
}

impl Format {
    /// Infer whether or not the output should be packed.
    pub fn infer_packed<P: AsRef<Path>>(file: P) -> bool {
        let file = file.as_ref();
        let ext = file.extension().and_then(OsStr::to_str);
        let stem = file.file_stem().and_then(OsStr::to_str);
        ext == Some("anim") || stem.map_or(false, |s| s.ends_with(".packed"))
    }

    /// Infer a format from given file name.
    pub fn infer<P: AsRef<Path>>(file: P) -> Option<Format> {
        match file.as_ref().extension()?.to_str()? {
            "txt" => Some(Format::Internal),
            "bin" => Some(Format::Bin),
            "anim" => Some(Format::Bin),
            "reanim" | "xml" => Some(Format::Xml),
            "json" => Some(Format::Json),
            "yaml" => Some(Format::Yaml),
            _ => None,
        }
    }
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Decode `.reanim.compiled` files.
    Decode {
        /// File name to open.
        file: PathBuf,
        /// Use structural format for output.
        #[clap(short, long)]
        packed: bool,
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
    pub fn run() -> anyhow::Result<()> {
        let args = Cli::parse();
        setup_logger(match args.verbose {
            None => LevelFilter::Warn, // no '--verbose', only errors
            Some(None) => LevelFilter::Info, // default '--verbose'
            Some(Some(verbose)) => verbose.into(), // explicit '--verbose'
        });
        match args.commands {
            Commands::Decode { file, packed, format, output } => {
                // open input & decode
                let file = File::open(&file).with_context(|| format!("failed to read file {file:?}"))?;
                let mut file = BufReader::new(file);
                let anim = Animation::decompress_and_decode(&mut file)?;

                // infer output format
                let inferred_format = output.as_ref().and_then(Format::infer);
                let format = format.or(inferred_format).unwrap_or(Format::Xml);
                let inferred_packed = output.as_ref().map_or(false, Format::infer_packed);
                let anim = if packed || inferred_packed {
                    MaybePacked::Packed(packed::AnimDesc::from(anim))
                } else {
                    MaybePacked::Plain(anim)
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
pub fn encode(anim: MaybePacked, format: Format, mut output: impl Write) -> anyhow::Result<()> {
    match (format, anim) {
        (Format::Internal, anim) => writeln!(output, "{anim:#?}")?,
        (Format::Bin, MaybePacked::Packed(anim)) => {
            bincode::encode_into_std_write(anim, &mut output, bincode::config::standard())?;
        }
        (Format::Xml, MaybePacked::Plain(anim)) => write!(output, "{}", Xml(anim))?,
        (Format::Json, anim) => serde_json::to_writer_pretty(output, &anim)?,
        (Format::Yaml, anim) => serde_yaml::to_writer(output, &anim)?,
        (format, anim) => log::error!(
            "format '{format}' does not support 'packed={}'",
            matches!(anim, MaybePacked::Packed(_))),
    }
    Ok(())
}
