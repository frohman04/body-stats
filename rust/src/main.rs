extern crate calamine;
use calamine::{open_workbook, DeError, RangeDeserializerBuilder, Reader, Xlsx, XlsxError};

extern crate chrono;
use chrono::{Date, Utc};

extern crate clap;
use clap::{App, Arg};

extern crate serde;
use serde::Deserialize;

extern crate tempfile;
use tempfile::NamedTempFile;

use std::fs::{copy, remove_file};
use std::path::Path;

fn main() {
    let matches = App::new("body-graphs")
        .version("0.1")
        .author("Chris Lieb")
        .arg(Arg::with_name("<file>").required(true).index(1))
        .get_matches();
    let raw_input_path = matches.value_of("<file>").unwrap();
    let input_path = Path::new(raw_input_path);

    if !input_path.exists() {
        panic!("Argument <file> ({}) does not exist", raw_input_path);
    } else if !input_path.is_file() {
        panic!("Argument <file> ({}) is not a file", raw_input_path);
    }

    let records = read_file(&input_path);
    println!("{}", raw_input_path);
}

/// Read the *.xlsx file and convert it into records.
fn read_file(path: &Path) -> Result<Vec<Record>, ReadError> {
    let temp_file = NamedTempFile::new()?;
    copy(path, temp_file.path())?;

    let mut workbook: Xlsx<_> = open_workbook(&temp_file)?;
    let range = workbook
        .worksheet_range("Weight")
        .ok_or(DeError::Custom("Unable to find sheet Weight".to_string()))??;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    let records = iter.map(|row| row.map(|x| Record::from(x)));

    let errors: Vec<ReadError> = records
        .filter(|x| x.is_err())
        .map(|x| ReadError::from(x.unwrap_err()))
        .collect();
    let out = if errors.len() == 0 {
        Result::Ok(records.map(|x| x.unwrap()).collect())
    } else {
        Result::Err(ReadError::from(errors[0]))
    };

    remove_file(temp_file)?;

    out
}

enum ReadError {
    Io { err: std::io::Error },
    Excel { err: XlsxError },
    Deserialize { err: DeError },
}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        ReadError::Io { err }
    }
}

impl From<XlsxError> for ReadError {
    fn from(err: XlsxError) -> Self {
        ReadError::Excel { err }
    }
}

impl From<DeError> for ReadError {
    fn from(err: DeError) -> Self {
        ReadError::Deserialize { err }
    }
}

#[derive(Deserialize, Debug)]
struct Record {
    date: Date<Utc>,
    weight: Option<f32>,
    fat_weight: Option<f32>,
    pct_fat: Option<f32>,
    pct_water: Option<f32>,
    pct_bone: Option<f32>,
    bmi: Option<f32>,
}
