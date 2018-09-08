#[macro_use]
extern crate clap;
extern crate regex;

use std::error;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::collections::HashSet;
use clap::Arg;

type Result<T> = std::result::Result<T, Box<error::Error>>;

pub enum RunInput {
    Stdin,
    File(String),
    Data(Vec<u8>),
}
pub struct RunConfig {
    pub fields: Vec<usize>,
    pub sorted: bool,
    pub whitespace: bool,
    pub input: RunInput,
}
impl Default for RunConfig {
    fn default() -> Self {
        let fields = vec![0];
        RunConfig {
            fields,
            sorted: false,
            whitespace: false,
            input: RunInput::Stdin,
        }
    }
}

fn main() -> Result<()> {
    let config = get_config()?;
    let mut out = io::stdout();

    // Compile-time specialisation based on input type
    // (You can still pass a Box<io::Read> for runtime flexibility)
    match config.input {
        RunInput::Stdin => {
            run(&config, &mut io::stdin(), &mut out)
        },
        RunInput::File(ref filename) => {
            run(&config, &mut fs::File::open(filename)?, &mut out)
        },
        RunInput::Data(ref data) => {
            run(&config, &mut io::Cursor::new(data), &mut out)
        }
    }
}

fn run<R, W>(config: &RunConfig, input: &mut R, output: &mut W) -> Result<()>
where R: io::Read, W: io::Write {
    let delim = if config.whitespace { r"\s+" } else { r"\t" };
    let splitter = regex::bytes::Regex::new(delim)?;

    // Construct a HashSet to track previously seen values (if sorted not set)
    let mut seen = HashSet::new();
    let mut last : Option<Vec<u8>> = None;

    let mut reader = BufReader::new(input);
    let mut line : Vec<u8> = vec![];
    while let Ok(_) = reader.read_until(0x0A as u8, &mut line) {
        if line.is_empty() {
            // EOF
            break;
        }

        // Build sort key
        let key : Vec<u8> = {
            let mut fields = splitter.split(&line);
            let mut key : Vec<u8> = vec![];
            let mut last_idx = 0;

            for idx in &config.fields {
                if let Some(column) = fields.nth(idx - last_idx) {
                    key.append(&mut column.into());
                    last_idx = idx + 1;
                }
                else {
                    break;
                }
            }
            key
        };

        let should_print = if config.sorted {
            // Compare against previous value
            match last {
                Some(ref last_key) if *last_key == key => {
                    false
                }
                _ => {
                    last = Some(key);
                    true
                }
            }
        }
        else {
            // Print if wasn't present in seen set
            seen.insert(key)
        };

        if should_print {
            output.write_all(&line)?;
        }
        line.clear();
    }

    output.flush()?;

    Ok(())
}

fn get_config() -> Result<RunConfig> {
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
            .index(1)
            .help("Input filename (defaults to standard input)"))
        .get_matches();

    // Fields may be a CSV
    let field_spec = args.value_of("fields").unwrap_or("1");
    let fields = parse_field_spec(field_spec).unwrap_or_else(|ref e| {
        println!("Error parsing field index(es): {}", e.description());
        println!("{}", args.usage());
        ::std::process::exit(1);
    });

    let sorted = args.is_present("sorted");
    let whitespace = args.is_present("whitespace");
    let input = match args.value_of("FILENAME") {
        Some(filename) => RunInput::File(filename.to_owned()),
        None => RunInput::Stdin,
    };

    Ok(RunConfig { fields, sorted, whitespace, input })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let config = RunConfig { whitespace: true, ..RunConfig::default() };
        let mut input = io::Cursor::new(b"a 1 2\na 2 3\nb 1 2\n");
        let mut output = io::Cursor::new(Vec::new());
        run(&config, &mut input, &mut output).expect("run to succeed");
        assert_eq!(
            String::from_utf8(output.into_inner()).unwrap(),
            "a 1 2\nb 1 2\n",
        );
    }

    #[test]
    fn column_2() {
        let config = RunConfig { whitespace: true, fields: vec![1], ..RunConfig::default() };
        let mut input = io::Cursor::new(b"a 1 2\na 2 3\nb 1 2\n");
        let mut output = io::Cursor::new(Vec::new());
        run(&config, &mut input, &mut output).expect("run to succeed");
        assert_eq!(
            String::from_utf8(output.into_inner()).unwrap(),
            "a 1 2\na 2 3\n",
        );
    }
}
