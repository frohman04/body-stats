#![forbid(unsafe_code)]

extern crate calamine;
extern crate clap;
#[macro_use]
extern crate log;
extern crate simplelog;
extern crate tempfile;
#[macro_use]
extern crate time;

mod regression;
#[macro_use]
mod timed;

use calamine::{open_workbook, DeError, RangeDeserializerBuilder, Reader, Xlsx, XlsxError};
use clap::{App, Arg};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use tempfile::NamedTempFile;
use time::{Date, Duration};

use std::fs::{copy, remove_file, write};
use std::path::Path;

use regression::SimpleRegression;

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )])
    .unwrap();

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

    let records = timed!(
        "Reading file",
        (|| {
            let records_result = read_file(input_path);
            if records_result.is_err() {
                panic!("{:?}", records_result.unwrap_err());
            }

            let records = records_result.unwrap();
            validate_file(&records);

            info!(
                "Read {} records covering {} days",
                records.len(),
                (records.last().unwrap().date - records[0].date).whole_days()
            );

            records
        })
    );
    timed!("Drawing graph", (|| draw_weight_graph(&records)));
}

/// Read the *.xlsx file and convert it into records.
fn read_file(path: &Path) -> Result<Vec<Record>, ReadError> {
    info!("Reading from file: {}", path.to_str().unwrap());

    let temp_file = NamedTempFile::new()?;
    copy(path, temp_file.path())?;

    let mut workbook: Xlsx<_> = open_workbook(&temp_file)?;
    let range = workbook
        .worksheet_range("Weight")
        .ok_or_else(|| DeError::Custom("Unable to find sheet Weight".to_string()))??;

    let iter = RangeDeserializerBuilder::new().from_range(&range)?;

    let epoch = date!(1899 - 12 - 30); // Excel epoch
    let (records, errors): (Vec<_>, Vec<_>) = iter
        .map(|row| {
            row.map(|x| {
                #[allow(clippy::type_complexity)]
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
fn validate_file(records: &[Record]) {
    let errors: Vec<String> = (1..records.len())
        .filter_map(|i| {
            if (records[i - 1].date - records[i].date).whole_days() < 0 {
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
        panic!(
            "Found issues in data read from file: \n{}",
            errors.join("\n")
        );
    }
}

/// Render a graph of weight data to HTML
fn draw_weight_graph(records: &[Record]) {
    let raw = timed!(
        "Calculating raw weight series",
        (|| weight_raw_series(records))
    );
    let average = timed!(
        "Calculating average weight series",
        (|| weight_average_series(records, 30))
    );
    let loess = timed!(
        "Calculating LOESS weight series",
        (|| weight_loess_series(records, 30))
    );

    let raw_dates: Vec<&str> = raw.iter().map(|p| p.date.as_str()).collect();
    let raw_values: Vec<String> = raw.iter().map(|p| p.value.to_string()).collect();
    let average_dates: Vec<&str> = average.iter().map(|p| p.date.as_str()).collect();
    let average_values: Vec<String> = average.iter().map(|p| p.value.to_string()).collect();
    let loess_dates: Vec<&str> = loess.iter().map(|p| p.date.as_str()).collect();
    let loess_values: Vec<String> = loess.iter().map(|p| p.value.to_string()).collect();

    let html = format!(
        "<!DOCTYPE html>
<html>
    <head>
        <title>Weight History</title>
        <script src=\"https://cdn.plot.ly/plotly-1.54.1.min.js\"></script>
    </head>
    <body>
        <div id=\"chart\"></div>
        <script>
            (function () {{
                var data0 = {{
                    \"name\": \"Weight\",
                    \"x\": [\"{}\"],
                    \"y\": [{}],
                    \"mode\": \"markers\",
                    \"type\": \"scatter\"
                }};
                var data1 = {{
                    \"name\": \"Rolling Average\",
                    \"x\": [\"{}\"],
                    \"y\": [{}],
                    \"type\": \"scatter\"
                }};
                var data2 = {{
                    \"name\": \"LOESS (SR)\",
                    \"x\": [\"{}\"],
                    \"y\": [{}],
                    \"type\": \"scatter\"
                }};

                var data = [data0, data1, data2];
                var layout = {{
                    \"yaxis\": {{
                        \"title\": \"Weight (lbs)\"
                    }},
                    \"xaxis\": {{
                        \"title\": \"Date\"
                    }}
                }};
                Plotly.plot(\"chart\", data, layout);
            }})();
        </script>
    </body>
</html>",
        raw_dates.join("\",\""),
        raw_values.join(","),
        average_dates.join("\",\""),
        average_values.join(","),
        loess_dates.join("\",\""),
        loess_values.join(",")
    );

    write("weight.html", html).expect("Unable to write file");
}

/// Calculate the data points for the raw weight series.
fn weight_raw_series(records: &[Record]) -> Vec<DataPoint> {
    records
        .iter()
        .filter_map(|r| {
            r.weight.map(|w| DataPoint {
                date: r.date.format("%Y-%m-%d"),
                value: w as f64,
            })
        })
        .collect()
}

/// Calculate the data points for the rolling average weight series.
fn weight_average_series(records: &[Record], num_days: i64) -> Vec<DataPoint> {
    let records: Vec<&Record> = records.iter().filter(|r| r.weight.is_some()).collect();

    let mut lower_init = 0;

    records
        .iter()
        .map(|r| {
            let lower_bound = r.date - Duration::days(num_days / 2);
            let upper_bound = r.date + Duration::days((num_days - 1) / 2);

            let mut count: i32 = 0;
            let mut sum: f64 = 0f64;

            let mut i = lower_init;
            while (lower_bound - records[i].date).whole_days() > 0 {
                i += 1;
            }
            lower_init = i;

            while i < records.len() && (records[i].date - upper_bound).whole_days() <= 0 {
                count += 1;
                sum += records[i].weight.unwrap() as f64;
                i += 1;
            }

            DataPoint {
                date: r.date.format("%Y-%m-%d"),
                value: sum / (count as f64),
            }
        })
        .collect()
}

/// Calculate the data points for the LOESS regression weight series.
fn weight_loess_series(records: &[Record], num_days: i64) -> Vec<DataPoint> {
    let records: Vec<&Record> = records.iter().filter(|r| r.weight.is_some()).collect();

    let base_date = records.iter().map(|r| r.date).min().unwrap();
    let mut lower_init = 0;

    records
        .iter()
        .map(|r| {
            let lower_bound = r.date - Duration::days(num_days / 2);
            let upper_bound = r.date + Duration::days((num_days - 1) / 2);

            let mut regression = SimpleRegression::new();

            let mut i = lower_init;
            while (lower_bound - records[i].date).whole_days() > 0 {
                i += 1;
            }
            lower_init = i;

            while i < records.len() && (records[i].date - upper_bound).whole_days() <= 0 {
                regression.add_data(
                    (records[i].date - base_date).whole_days() as f64,
                    records[i].weight.unwrap() as f64,
                );
                i += 1;
            }

            DataPoint {
                date: r.date.format("%Y-%m-%d"),
                value: regression.predict((r.date - base_date).whole_days() as f64),
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
    date: Date,
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
