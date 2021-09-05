extern crate csv;

use csv::{Position, Reader};
use std::fs::File;

fn string_record_to_vec(record: &csv::StringRecord) -> Vec<String> {
    let mut string_vec= Vec::new();
    for field in record.iter() {
        string_vec.push(String::from(field));
    }
    string_vec
}

pub struct CsvLensReader {
    reader: Reader<File>,
    pub headers: Vec<String>,
}

impl CsvLensReader {

    pub fn new(filename: &str) -> Self {
        let mut reader = Reader::from_path(filename).unwrap();
        let headers_record = reader.headers().unwrap();
        let headers = string_record_to_vec(headers_record);
        Self {
            reader: reader,
            headers: headers,
        }
    }

    pub fn get_rows(&mut self, rows_from: u64, num_rows: u64) -> csv::Result<Vec<Vec<String>>> {

        let pos = Position::new();
        self.reader.seek(pos)?;

        let records = self.reader.records();
        let mut res = Vec::new();

        let rows_to = rows_from + num_rows;

        for (i, r) in records.enumerate() {
            // TODO: always assume has header
            if i == 0 {
                continue;
            }
            // rows_from is 0-based
            let i = i - 1;
            if i >= rows_from as usize && i < rows_to as usize {
                let string_record = r.unwrap();
                let mut row = Vec::new();
                for field in string_record.iter() {
                    row.push(String::from(field));
                }
                res.push(row);
            }

            if i >= rows_to as usize {
                break;
            }

        }
        Ok(res)
    }
}