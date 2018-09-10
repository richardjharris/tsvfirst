#[macro_use]
extern crate clap;

use std::error;
use std::io;
use clap::Arg;

mod config;
mod tsvfirst;

use config::Config;

type Result<T> = std::result::Result<T, Box<error::Error>>;

fn main() -> Result<()> {
    let config = get_config()?;
    let mut out = io::stdout();
    tsvfirst::run(&config, &mut out)
}

fn get_config() -> Result<Config> {
    let args = app_from_crate!()
        .usage("tsvfirst [-f 1,2] [-s] [-w] <file or stdin>")
        .arg(Arg::with_name("fields")
            .short("f")
            .long("fields")
            .alias("field")
            .takes_value(true)
            .value_name("SPEC")
            .help("Index(es) of fields to unique by, e.g '1' or '2,3' [default: 1]")
            .long_help(
"One or more columns to use when determining the uniqueness of a row. Columns
are specified by their number, starting from column 1. Multiple columns should
be joined with a comma."))

        .arg(Arg::with_name("whitespace")
            .long("whitespace")
            .short("w")
            .help("Split fields whitespace instead of tabs"))

        .arg(Arg::with_name("sorted")
            .long("sorted")
            .short("s")
            .help("Assume the input is sorted by the fields in -f (faster)")
            .long_help(
"This asserts that the input file is pre-sorted by the fields specified by the
-f argument (or more accurately: that all rows with the same combination of values
for those fields appear consecutively). This is faster because tsvfirst only needs
to compare the previous and current rows to determine uniqueness, rather than
tracking all previously seen values."))

        .arg(Arg::with_name("FILENAME")
            .multiple(true)
            .help("Input filename/s (defaults to standard input)")
            .long_help(
"One or more filenames to use as input: all files will be processed in order
as if concatenated. If no filenames specified, defaults to standard input.
The filename of '-' (a single dash) is also taken to mean standard input."))
        .get_matches();

    // Fields may be a CSV
    let field_spec = args.value_of("fields").unwrap_or("1");
    let fields = parse_field_spec(field_spec).unwrap_or_else(|ref e| {
        println!("Error parsing field index(es): {}", e.description());
        println!("{}", args.usage());
        ::std::process::exit(1);
    });

    let mut config = Config::new()
        .fields(&fields)
        .sorted(args.is_present("sorted"))
        .whitespace(args.is_present("whitespace"));

    if let Some(inputs) = args.values_of("FILENAME") {
        for input in inputs {
            config = config.add_input(input);
        }
    }

    Ok(config)
}

fn parse_field_spec(arg: &str) -> Result<Vec<usize>> {
    let mut fields = vec![];
    for field in arg.split(',') {
        let field = field.parse::<usize>()?;
        if field == 0 {
            return Err("output field is 1-indexed; 0 is not valid".into());
        }
        // Convert to 0-indexed
        fields.push(field - 1);
    }

    if fields.is_empty() {
        return Err("no fields specified".into());
    }

    fields.sort();
    fields.dedup();
    Ok(fields)
}
