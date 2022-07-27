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
use clap::{ValueEnum, Parser, Subcommand};
use fern::colors::{Color::*, ColoredLevelConfig};
use log::LevelFilter;
use serde::{Serialize, Serializer};
use libre_pvz_resources::animation as packed;
use libre_pvz_resources::model;
use crate::reanim::Animation;
use crate::xml::Xml as XmlWrapper;

/// Optionally packed animations.
pub enum MaybePacked {
    /// Plain format, structurally equivalent to reanim XML.
    Plain(Animation),
    /// Packed format, compact and structural.
    Packed(packed::AnimDesc),
}

use MaybePacked::*;

impl MaybePacked {
    /// Is this already packed?
    pub fn is_packed(&self) -> bool { matches!(self, Packed(_)) }

    /// Force this to be packed, fail if unpacking is requested.
    pub fn into_packed(self, packed: bool) -> anyhow::Result<MaybePacked> {
        Ok(match self {
            Packed(_) if !packed => anyhow::bail!("unpacking not supported"),
            Plain(anim) if packed => Packed(anim.into()),
            _ => self,
        })
    }
}

impl Debug for MaybePacked {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Plain(anim) => anim.fmt(f),
            Packed(anim) => anim.fmt(f),
        }
    }
}

impl Serialize for MaybePacked {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Plain(anim) => anim.serialize(serializer),
            Packed(anim) => anim.serialize(serializer),
        }
    }
}

/// Entry of the command line interface.
#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Verbosity, for filtering diagnostics messages.
    #[clap(long, value_enum, global = true)]
    pub verbose: Option<Option<Verbosity>>,
    /// All available subcommands.
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Output format: JSON and YAML supported, guarded by crate features.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Format {
    /// Internal encoding (Rust `{:#?}` debug pretty printing)
    Internal,
    /// Original "compiled format": file extension `.reanim.compiled`.
    Compiled,
    /// Binary encoding using `bincode`. Only support packed format.
    Bin,
    /// XML format as is used in original PvZ game.
    Xml,
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
}

use Format::*;

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Internal => "internal",
            Compiled => "compiled",
            Bin => "bin",
            Xml => "xml",
            Json => "json",
            Yaml => "yaml",
        })
    }
}

impl Format {
    /// Infer whether or not the output should be packed.
    pub fn infer_packed<P: AsRef<Path>>(path: P) -> bool {
        let file = path.as_ref();
        let ext = file.extension().and_then(OsStr::to_str);
        let stem = file.file_stem().and_then(OsStr::to_str);
        ext == Some("anim") || stem.map_or(false, |s| s.ends_with(".packed"))
    }

    /// Infer a format from given file name.
    pub fn infer<P: AsRef<Path>>(path: P) -> Option<Format> {
        match path.as_ref().extension()?.to_str()? {
            "txt" => Some(Internal),
            "compiled" => Some(Compiled),
            "bin" => Some(Bin),
            "anim" => Some(Bin),
            "reanim" | "xml" => Some(Xml),
            "json" => Some(Json),
            "yaml" => Some(Yaml),
            _ => None,
        }
    }

    /// Decide an input/output format for a given (option, path, default) tuple.
    pub fn decide<P: AsRef<Path>>(spec: Option<Format>, path: Option<P>, default: Format) -> Format {
        spec.or_else(|| path.and_then(|p| Format::infer(p.as_ref()))).unwrap_or(default)
    }
}

/// Subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Conversion for model files.
    Model {
        /// Input file path.
        input: PathBuf,
        /// Input format.
        #[clap(short = 'I', long, value_enum)]
        input_format: Option<Format>,
        /// Output file path.
        #[clap(short, long)]
        output: Option<PathBuf>,
        /// Output format.
        #[clap(short = 'O', long, value_enum)]
        output_format: Option<Format>,
    },
    /// Conversion for animation files.
    Anim {
        /// Input file path.
        input: PathBuf,
        /// Input format.
        #[clap(short = 'I', long, value_enum)]
        input_format: Option<Format>,
        /// Use structural format for input.
        #[clap(long)]
        pack_input: bool,
        /// Output file path.
        #[clap(short, long)]
        output: Option<PathBuf>,
        /// Output format.
        #[clap(short = 'O', long, value_enum)]
        output_format: Option<Format>,
        /// Use structural format for output.
        #[clap(long)]
        pack_output: bool,
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

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

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
            Commands::Model {
                input, input_format,
                output_format, output,
            } => {
                // open input & decode
                let input_format = Format::decide(input_format, Some(&input), Bin);
                let input = File::open(&input).with_context(|| format!("failed to read file {input:?}"))?;
                let mut input = BufReader::new(input);
                let model: model::Model = match input_format {
                    Internal | Compiled | Xml => anyhow::bail!("unsupported input format: {input_format}"),
                    Bin => bincode::decode_from_std_read(&mut input, BINCODE_CONFIG)?,
                    Json => serde_json::from_reader(&mut input)?,
                    Yaml => serde_yaml::from_reader(&mut input)?,
                };

                // infer output format
                let output_format = Format::decide(output_format, output.as_ref(), Internal);
                // output file (or stdout)
                if let Some(output) = output {
                    let context = || format!("failed to open output file {output:?}");
                    let output = File::create(&output).with_context(context)?;
                    encode_model(model, output_format, output)?;
                } else {
                    encode_model(model, output_format, std::io::stdout().lock())?;
                }
            }
            Commands::Anim {
                input, input_format, mut pack_input,
                output_format, output, mut pack_output,
            } => {
                // open input & decode
                pack_input |= Format::infer_packed(&input);
                let input_format = Format::decide(input_format, Some(&input), Compiled);
                let input = File::open(&input).with_context(|| format!("failed to read file {input:?}"))?;
                let mut input = BufReader::new(input);
                let anim = match input_format {
                    Internal | Xml => anyhow::bail!("unsupported input format: {input_format}"),
                    Bin => Packed(bincode::decode_from_std_read(&mut input, BINCODE_CONFIG)?),
                    Compiled => Plain(Animation::decompress_and_decode(&mut input)?),
                    Json if pack_input => Packed(serde_json::from_reader(&mut input)?),
                    Yaml if pack_input => Packed(serde_yaml::from_reader(&mut input)?),
                    Json => Plain(serde_json::from_reader(&mut input)?),
                    Yaml => Plain(serde_yaml::from_reader(&mut input)?),
                };

                // infer output format
                pack_output |= output.as_ref().map_or(anim.is_packed(), Format::infer_packed);
                let output_format = Format::decide(
                    output_format, output.as_ref(),
                    if pack_output { Internal } else { Xml },
                );
                let anim = anim.into_packed(pack_output)?;

                // output file (or stdout)
                if let Some(output) = output {
                    let context = || format!("failed to open output file {output:?}");
                    let output = File::create(&output).with_context(context)?;
                    encode_anim(anim, output_format, output)?;
                } else {
                    encode_anim(anim, output_format, std::io::stdout().lock())?;
                }
            }
        }
        Ok(())
    }
}

/// Encode the animation into required format.
pub fn encode_anim(anim: MaybePacked, format: Format, mut output: impl Write) -> anyhow::Result<()> {
    match (format, anim) {
        (Internal, anim) => writeln!(output, "{anim:#?}")?,
        (Bin, Packed(anim)) => {
            bincode::encode_into_std_write(anim, &mut output, BINCODE_CONFIG)?;
        }
        (Xml, Plain(anim)) => write!(output, "{}", XmlWrapper(anim))?,
        (Json, anim) => serde_json::to_writer_pretty(output, &anim)?,
        (Yaml, anim) => serde_yaml::to_writer(output, &anim)?,
        (format, anim) => {
            anyhow::bail!("format '{format}' does not support 'packed={}'", anim.is_packed());
        }
    }
    Ok(())
}

/// Encode the model into required format.
pub fn encode_model(model: model::Model, format: Format, mut output: impl Write) -> anyhow::Result<()> {
    match format {
        Compiled | Xml => anyhow::bail!("unsupported output format: '{format}'"),
        Internal => writeln!(output, "{model:#?}")?,
        Bin => { bincode::encode_into_std_write(&model, &mut output, BINCODE_CONFIG)?; }
        Json => serde_json::to_writer_pretty(output, &model)?,
        Yaml => serde_yaml::to_writer(output, &model)?,
    }
    Ok(())
}
