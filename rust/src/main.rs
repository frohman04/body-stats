extern crate clap;
use clap::{Arg, App};

use std::path::Path;

fn main() {
    let matches = App::new("body-graphs")
        .version("0.1")
        .author("Chris Lieb")
        .arg(Arg::with_name("<file>")
            .required(true)
            .index(1))
        .get_matches();
    let raw_input_path = matches.value_of("<file>").unwrap();
    let input_path = Path::new(raw_input_path);

    if !input_path.exists() {
        panic!("Argument <file> ({}) does not exist", raw_input_path);
    } else if !input_path.is_file() {
        panic!("Argument <file> ({}) is not a file", raw_input_path);
    }

    println!("{}", raw_input_path);
}
