extern crate regex;

use std::io;
use std::collections::HashSet;
use std::error;

use config::Config;

pub fn run<W>(config: &Config, output: &mut W) -> Result<(), Box<error::Error>>
where W: io::Write {
    let delim = if config.whitespace { r"\s+" } else { r"\t" };
    let splitter = regex::bytes::Regex::new(delim)?;

    // Construct a HashSet to track previously seen values (if sorted not set)
    let mut seen = HashSet::new();
    let mut last : Option<Vec<u8>> = None;

    let mut reader = config.get_reader()?;
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

