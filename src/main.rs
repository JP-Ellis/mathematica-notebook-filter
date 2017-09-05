// mathematica-notebook-filter
// Copyright (C) 2017  Joshua Ellis <josh@jpellis.me>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <http://www.gnu.org/licenses/>.

extern crate atty;
#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate tempfile;

use std::fs;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path;
use std::process::exit;

use clap::{App, Arg, AppSettings};

mod parser;

const ABOUT: &'static str = "
mathematica-notebook-filter parses Mathematica notebook files and strips them of superfluous information so that they can be committed into version control systems more easily.

By default, mathematica-notebook-filter will read from the standard input and write to the standard output.

Project home page: https://github.com/JP-Ellis/mathematica-notebook-filter
";

/// Create the `clap::App` which will be used to parse the arguments.
fn app() -> App<'static, 'static> {
    App::new("mathematica-notebook-filter")
        .author(crate_authors!())
        .version(crate_version!())
        .max_term_width(100)
        .setting(AppSettings::UnifiedHelpMessage)
        .about(ABOUT)
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .takes_value(true)
                .default_value("-")
                .number_of_values(1)
                .allow_hyphen_values(true)
                .help("Input file name, or standard input if not provided.")
                .long_help(
                    "Specify the input file to the Mathematica Notebook which\
                     will parsed.  This value is option and will default to the\
                     standard input.  The standard input can be explicitly\
                     specified using the special file name '-'.",
                ),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .takes_value(true)
                .default_value("-")
                .number_of_values(1)
                .allow_hyphen_values(true)
                .help("Output file name, or standard output if not provided.")
                .long_help(
                    "Specify the output file to which the Mathematica Notebook\
                     stripped of all output will be written.  This value is\
                     option and will default to the standard output.  The\
                     standard output can be explicitly specified using the\
                     special file name '-'.

\
                     If the input and output files are both the same, the output\
                     will be written to a temporary file first before\
                     overwriting the input file.

\
                     If the output file already exists, the contents of the file\
                     will be overwritten.",
                ),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .takes_value(false)
                .help("Increase the verbosity of errors.")
                .long_help(
                    "Increase the verbosity of errors.  The errors are\
                     outputted to the standard error stream and thus do not\
                     appear in the output.  This option can be specified\
                     multiple times for increasing levels of verbosity.",
                ),
        )
}

/// Initialize the logger based on the desired level of verbosity.
fn initialize_logger(level: u64) {
    let mut log_builder = env_logger::LogBuilder::new();
    match level {
        0 => log_builder.filter(None, log::LogLevelFilter::Error),
        1 => log_builder.filter(None, log::LogLevelFilter::Warn),
        2 => log_builder.filter(None, log::LogLevelFilter::Info),
        3 | _ => log_builder.filter(None, log::LogLevelFilter::Debug),
    };
    log_builder.format(|record: &log::LogRecord| {
        format!("{}: {}", record.level(), record.args())
    });
    if let Err(e) = log_builder.init() {
        eprintln!("Error when initializing the logger: {}.", e);
        exit(1);
    }

    debug!("Verbosity set to Debug.");
}

/// Main function
fn main() {
    // Parse the arguments, and immediately initialize the logger.
    let matches = app().get_matches();
    initialize_logger(matches.occurrences_of("verbose"));

    // Check if the input and output files are the same (and not both standard
    // input/output) as we need to take care of not overwriting the input as we
    // write to it.
    let use_temporary_file = matches.value_of("input") != Some("-") &&
        matches.value_of("input") == matches.value_of("output");

    // We initialize stdin and stdout here so that they don't go out of scope
    // (even if they don't get used).  This doesn't seem like the best way of
    // doing this...
    let stdin = io::stdin();
    let stdout = io::stdout();

    // Get the input file.
    let mut input_file: Box<BufRead> = match matches.value_of("input") {
        Some("-") | None => {
            if atty::is(atty::Stream::Stdin) {
                error!("Cowardly exiting as standard input is a tty.");
                exit(1);
            }

            debug!("Reading from standard input.");

            Box::new(stdin.lock())
        }
        Some(filename) => {
            debug!("Reading from: {}.", filename);

            let file = match fs::File::open(filename) {
                Ok(file) => file,
                Err(e) => {
                    error!("Error when opening input file: {}.", e);
                    exit(1)
                }
            };
            Box::new(BufReader::new(file))
        }
    };

    // Get the output file and output filename if it exists since we might need
    // to rename the output file and overwrite the input file if both input and
    // output files were identical.
    let (mut output_file, output_filename): (Box<Write>, _) =
        match (matches.value_of("output"), use_temporary_file) {
            (_, true) => {
                debug!("Creating temporary output file.");

                // Using a temporary file.  As we will wish to rename it (and
                // the temporary file is not used to store any sensitive
                // information), we use a NamedTempFile.
                let output_file = match tempfile::NamedTempFile::new() {
                    Ok(file) => file,
                    Err(e) => {
                        error!("Error when creating temporary file: {}.", e);
                        exit(1)
                    }
                };
                let p = output_file.path().to_path_buf();

                debug!("Writing to temporary file: {:?}.", p);

                (Box::new(output_file), Some(p))
            }

            (Some("-"), false) |
            (None, false) => {
                debug!("Writing to standard output.");

                (Box::new(stdout.lock()), None)
            }

            (Some(filename), false) => {
                debug!("Writing to: {}.", filename);

                let p = path::Path::new(filename);
                if p.is_file() {
                    warn!("Overwriting output file: {}.", filename);
                }
                let file = match fs::File::create(p) {
                    Ok(file) => file,
                    Err(e) => {
                        error!("Error when opening output file: {}.", e);
                        exit(1)
                    }
                };

                (Box::new(file), Some(p.to_path_buf()))
            }
        };

    if let Err(e) = parser::parse_input(&mut input_file, &mut output_file) {
        error!("Error when parsing notebook: {}.", e);
        exit(1);
    }

    // Before exiting, we just need to overwrite the input file.
    if use_temporary_file {
        let input_filename = matches.value_of("input").unwrap();
        let output_filename = output_filename.unwrap();
        debug!("Moving {:?} => {}", output_filename, input_filename);

        if let Err(e) = fs::copy(output_filename, input_filename) {
            error!(
                "Error when replacing original input with temporary outout: {}",
                e
            );
            exit(1)
        } else {
            exit(0)
        }
    }
}
