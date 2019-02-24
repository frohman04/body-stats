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

mod regression;
use regression::SimpleRegression;

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
    let raw_weights = weight_raw_series(&records);
    let average_weights = weight_average_series(&records, 30);
    let loess_weights = weight_loess_series(&records, 30);
    for record in loess_weights {
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

/// Calculate the data points for the raw weight series.
fn weight_raw_series(records: &Vec<Record>) -> Vec<DataPoint> {
    records
        .into_iter()
        .filter_map(|r| {
            r.weight.map(|w| DataPoint {
                date: r.date.format("%Y-%m-%d").to_string(),
                value: w as f64,
            })
        })
        .collect()
}

/// Calculate the data points for the rolling average weight series.
fn weight_average_series(records: &Vec<Record>, num_days: i64) -> Vec<DataPoint> {
    let records: Vec<&Record> = records.into_iter().filter(|r| r.weight.is_some()).collect();

    let mut lower_init = 0;

    records
        .iter()
        .map(|r| {
            let lower_bound = r.date - Duration::days(num_days / 2);
            let upper_bound = r.date + Duration::days((num_days - 1) / 2);

            let mut count: i32 = 0;
            let mut sum: f64 = 0f64;

            let mut i = lower_init;
            while lower_bound
                .signed_duration_since(records[i].date)
                .num_days()
                > 0
            {
                i += 1;
            }
            lower_init = i;

            while i < records.len()
                && records[i]
                    .date
                    .signed_duration_since(upper_bound)
                    .num_days()
                    <= 0
            {
                count += 1;
                sum += records[i].weight.unwrap() as f64;
                i += 1;
            }

            DataPoint {
                date: r.date.format("%Y-%m-%d").to_string(),
                value: sum / (count as f64),
            }
        })
        .collect()
}

/// Calculate the data points for the LOESS regression weight series.
fn weight_loess_series(records: &Vec<Record>, num_days: i64) -> Vec<DataPoint> {
    let records: Vec<&Record> = records.into_iter().filter(|r| r.weight.is_some()).collect();

    let base_date = records.iter().map(|r| r.date).min().unwrap();
    let mut lower_init = 0;

    records
        .iter()
        .map(|r| {
            let lower_bound = r.date - Duration::days(num_days / 2);
            let upper_bound = r.date + Duration::days((num_days - 1) / 2);

            let mut regression = SimpleRegression::new();

            let mut i = lower_init;
            while lower_bound
                .signed_duration_since(records[i].date)
                .num_days()
                > 0
            {
                i += 1;
            }
            lower_init = i;

            while i < records.len()
                && records[i]
                    .date
                    .signed_duration_since(upper_bound)
                    .num_days()
                    <= 0
            {
                regression.add_data(
                    records[i].date.signed_duration_since(base_date).num_days() as f64,
                    records[i].weight.unwrap() as f64,
                );
                i += 1;
            }

            DataPoint {
                date: r.date.format("%Y-%m-%d").to_string(),
                value: regression
                    .predict(r.date.signed_duration_since(base_date).num_days() as f64),
            }
        })
        .collect()
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

#[derive(Debug)]
struct DataPoint {
    date: String,
    value: f64,
}
