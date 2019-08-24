extern crate lazy_static;
extern crate regex;

use std::io::{self, stdout};
use structopt::StructOpt;

pub mod pipem;
use crate::pipem::{OutputTemplate, merge_input};

#[derive(StructOpt)]
struct Cli {
    template: String,
    #[structopt(short = "F", long = "field-separator", default_value = " ")]
    field_separator: String,
    #[structopt(short = "R", long = "record-separator", default_value = "\n")]
    record_separator: String
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();
    let raw_template = args.template.as_str();
    let field_separator: u8 = args.field_separator.as_str().as_bytes()[0];
    let record_separator: u8 = args.record_separator.as_str().as_bytes()[0];

    let template = OutputTemplate::parse(raw_template, field_separator, record_separator);
    let stdin = io::stdin();
    merge_input(stdin.lock(), &mut stdout().lock(), template)?;
    Ok(())
}
