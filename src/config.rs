use std::io::prelude::*;
use std::io;
use std::io::BufReader;
use std::fs;

#[derive(Debug)]
pub struct Config {
    pub inputs: Vec<String>,  // empty implies stdin
    pub fields: Vec<usize>,
    pub sorted: bool,
    pub whitespace: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            inputs: vec![],
            fields: vec![1],
            sorted: false,
            whitespace: false,
        }
    }

    pub fn add_input(mut self, input: &str) -> Config {
        self.inputs.push(input.into());
        self
    }

    pub fn fields(mut self, fields: &[usize]) -> Config {
        self.fields = fields.to_owned();
        self
    }

    pub fn sorted(mut self, yes: bool) -> Config {
        self.sorted = yes;
        self
    }

    pub fn whitespace(mut self, yes: bool) -> Config {
        self.whitespace = yes;
        self
    }

    pub fn get_reader(&self) -> io::Result<Box<io::BufRead>> {
        let default_input = vec!["-".into()];
        let inputs = if self.inputs.is_empty() {
            &default_input
        }
        else {
            &self.inputs
        };

        let mut reader : Option<Box<BufRead>> = None;
        let mut stdin_used = false;
        for input in inputs {
            let sub_reader : Box<BufRead> = match input.as_ref() {
                "-" => if stdin_used {
                        return Err(io::Error::new(io::ErrorKind::Other, "stdin used twice"));
                    }
                    else {
                        stdin_used = true;
                        // Experimental: lock io::stdin() for duration of program
                        // Should be fine as it's a few bytes
                        let stdin = Box::leak(Box::new(io::stdin()));
                        Box::new(stdin.lock())
                    },
                filename => Box::new(BufReader::new(fs::File::open(filename)?)),
            };
            reader = match reader {
                None => Some(sub_reader),
                Some(reader) => Some(Box::new(reader.chain(sub_reader))),
            }
        }
        Ok(reader.unwrap())
    }
}
