extern crate lazy_static;
extern crate regex;

use std::io::{self, stdout};
use structopt::StructOpt;

pub mod pipem;
use crate::pipem::{OutputTemplate, merge_input};

#[derive(StructOpt)]
struct Cli {
    template: String,
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();
    let raw_template = args.template.as_str();
    let template = OutputTemplate::parse(raw_template);

//    let input: &[u8] = b"one1 one2 one3\ntwo1 two2 two3\nthree1 three2 three3";
//    let cursor = io::Cursor::new(input);
//    merge_input(cursor, &mut stdout().lock(), template)?;



    let stdin = io::stdin();
    merge_input(stdin.lock(), &mut stdout().lock(), template)?;

    Ok(())
}
