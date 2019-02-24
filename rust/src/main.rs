extern crate calamine;
use calamine::{open_workbook, DeError, RangeDeserializerBuilder, Reader, Xlsx, XlsxError};

extern crate chrono;
use chrono::prelude::*;

extern crate clap;
use clap::{App, Arg};

extern crate tempfile;
use tempfile::NamedTempFile;

extern crate time;
use time::Duration;

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

    println!("{}", raw_input_path);

    let records = read_file(&input_path);
    if records.is_err() {
        panic!(format!("{:?}", records.unwrap_err()));
    }

    let records = records.unwrap();
    validate_file(&records);
    for record in records {
        println!("{:?}", record);
    }
}

/// Read the *.xlsx file and convert it into records.
fn read_file(path: &Path) -> Result<Vec<Record>, ReadError> {
    let temp_file = NamedTempFile::new()?;
    copy(path, temp_file.path())?;

    let mut workbook: Xlsx<_> = open_workbook(&temp_file)?;
    let range = workbook
        .worksheet_range("Weight")
        .ok_or_else(|| DeError::Custom("Unable to find sheet Weight".to_string()))??;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    iter.next(); // skip first row
    let epoch = Local.ymd(1899, 12, 30);
    let (records, errors): (Vec<_>, Vec<_>) = iter
        .map(|row| {
            row.map(|x| {
                let (days_since_epoch, _, weight, fat_weight, pct_fat, pct_water, pct_bone, bmi): (
                    f32,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                ) = x;
                Record {
                    date: epoch + Duration::days(days_since_epoch as i64),
                    weight,
                    fat_weight,
                    pct_fat,
                    pct_water,
                    pct_bone,
                    bmi,
                }
            })
            .map_err(ReadError::from)
        })
        .partition(Result::is_ok);
    let records: Vec<Record> = records.into_iter().map(Result::unwrap).collect();
    let errors: Vec<ReadError> = errors.into_iter().map(Result::unwrap_err).collect();

    remove_file(temp_file)?;
    if errors.is_empty() {
        Result::Ok(records)
    } else {
        Result::Err(errors.into_iter().next().unwrap())
    }
}

/// Validate that the rows in the input file are naturally in increasing date order
fn validate_file(records: &Vec<Record>) -> () {
    let errors: Vec<String> = (1..records.len())
        .filter_map(|i| {
            if records[i - 1]
                .date
                .signed_duration_since(records[i].date)
                .num_days()
                < 0
            {
                None
            } else {
                Some(format!(
                    "Date for row {} is the same or later than the date for row {}",
                    records[i - 1].date,
                    records[i].date
                ))
            }
        })
        .collect();
    if !errors.is_empty() {
        panic!(format!(
            "Found issues in data read from file: \n{}",
            errors.join("\n")
        ));
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
struct Record {
    date: Date<Local>,
    weight: Option<f32>,
    fat_weight: Option<f32>,
    pct_fat: Option<f32>,
    pct_water: Option<f32>,
    pct_bone: Option<f32>,
    bmi: Option<f32>,
}
