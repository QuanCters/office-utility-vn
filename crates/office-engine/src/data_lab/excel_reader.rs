use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};

use calamine::{open_workbook_auto_from_rs, Data, Reader};
use office_shared::CommandError;

/// Tự động đọc dữ liệu dạng bảng từ bytes (hỗ trợ .xlsx, .xls, .ods, .csv, .tsv)
pub fn read_tabular_rows(
    bytes: &[u8],
    cancel_flag: &AtomicBool,
) -> Result<Vec<Vec<String>>, CommandError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(CommandError::TaskCancelled);
    }

    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    // Kiểm tra magic bytes:
    // - PK\x03\x04 (ZIP, bao gồm .xlsx, .ods)
    // - \xD0\xCF\x11\xE0 (OLE2 Compound Document, .xls 97-2003)
    let is_spreadsheet = bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]);

    if is_spreadsheet {
        let cursor = Cursor::new(bytes);
        let mut workbook = match open_workbook_auto_from_rs(cursor) {
            Ok(wb) => wb,
            Err(e) => {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Không thể phân tích định dạng bảng tính: {e}"),
                });
            }
        };

        let sheet_names = workbook.sheet_names();
        let first_sheet = match sheet_names.first() {
            Some(s) => s.clone(),
            None => return Ok(Vec::new()),
        };

        let range = match workbook.worksheet_range(&first_sheet) {
            Ok(r) => r,
            Err(e) => {
                return Err(CommandError::ValidationFailed {
                    reason: format!("Không thể đọc sheet '{first_sheet}': {e}"),
                });
            }
        };

        let mut rows = Vec::new();
        for row in range.rows() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let cells: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    Data::String(s) => s.clone(),
                    Data::Float(f) => {
                        if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                            format!("{}", *f as i64)
                        } else {
                            format!("{}", f)
                        }
                    }
                    Data::Int(i) => format!("{}", i),
                    Data::Bool(b) => format!("{}", b),
                    Data::DateTime(d) => format!("{}", d),
                    Data::DateTimeIso(s) => s.clone(),
                    Data::DurationIso(s) => s.clone(),
                    Data::Error(e) => format!("{:?}", e),
                })
                .collect();

            // Bỏ qua dòng trống hoàn toàn
            if cells.iter().all(|c| c.trim().is_empty()) {
                continue;
            }

            rows.push(cells);
        }

        Ok(rows)
    } else {
        // Xử lý tệp văn bản thuần (CSV, TSV)
        let text = String::from_utf8_lossy(bytes);
        let mut rows = Vec::new();

        for line in text.lines() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let raw = line.trim();
            if raw.is_empty() {
                continue;
            }

            let delimiter = if raw.contains('\t') {
                '\t'
            } else if raw.contains(';') {
                ';'
            } else {
                ','
            };

            let cols: Vec<String> = raw
                .split(delimiter)
                .map(|c| c.trim().to_string())
                .collect();

            rows.push(cols);
        }

        Ok(rows)
    }
}

/// Dựng nội dung CSV chuẩn UTF-8 từ bảng 2D
pub fn format_table_to_csv(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in rows {
        let line: Vec<String> = row
            .iter()
            .map(|cell| {
                if cell.contains(',') || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
                    format!("\"{}\"", cell.replace('"', "\"\""))
                } else {
                    cell.clone()
                }
            })
            .collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_csv_tabular_rows() {
        let cancel = AtomicBool::new(false);
        let csv_data = b"Name,Age,Role\nNguyen Van A,30,Developer\nTran Thi B,25,Designer\n";
        let rows = read_tabular_rows(csv_data, &cancel).expect("read csv");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["Name", "Age", "Role"]);
        assert_eq!(rows[1], vec!["Nguyen Van A", "30", "Developer"]);
    }

    #[test]
    fn test_read_tsv_tabular_rows() {
        let cancel = AtomicBool::new(false);
        let tsv_data = b"ID\tScore\tStatus\n101\t9.5\tPass\n102\t4.0\tFail\n";
        let rows = read_tabular_rows(tsv_data, &cancel).expect("read tsv");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1], vec!["101", "9.5", "Pass"]);
    }

    #[test]
    fn test_read_empty_tabular_rows() {
        let cancel = AtomicBool::new(false);
        let rows = read_tabular_rows(b"", &cancel).expect("read empty");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_format_table_to_csv() {
        let table = vec![
            vec!["Name".to_string(), "Notes".to_string()],
            vec!["Nguyen, A".to_string(), "Special \"quote\"".to_string()],
        ];
        let csv = format_table_to_csv(&table);
        assert!(csv.contains("\"Nguyen, A\""));
        assert!(csv.contains("\"Special \"\"quote\"\"\""));
    }

    #[test]
    fn test_cancellation() {
        let cancel = AtomicBool::new(true);
        let res = read_tabular_rows(b"A,B,C\n1,2,3", &cancel);
        assert_eq!(res, Err(CommandError::TaskCancelled));
    }
}
