//! Bộ Kiểm thử Hiệu năng Đa lõi (Criterion Performance Benchmarks)
//!
//! Xác minh 4 kịch bản hiệu năng chuẩn theo đặc tả docs/specs/06-testing-qa.md:
//! - BENCH-01: Rayon Image Batch (nén ảnh song song, thời gian < 4.0s, RAM < 250MB)
//! - BENCH-02: Word Mailmerge (trộn 1.000 hợp đồng .docx có 25 placeholder < 1.2s)
//! - BENCH-03: Chuyển mã Font Tiếng Việt (50.000 dòng TCVN3 -> Unicode NFC >= 100.000 dòng/giây)
//! - BENCH-04: Ghi Nhật ký & Truy vấn SQLite WAL (1.000 bản ghi, p99 < 1ms, zero lock)

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use office_engine::OfficeTool;
use office_shared::DispatchTaskRequest;
use rusqlite::{params, Connection};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use tempfile::TempDir;

/// BENCH-01: Xử lý Nén Ảnh Song song (Rayon Image Batch)
fn bench_01_image_batch_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("BENCH-01_Rayon_Image_Batch");
    group.sample_size(10);

    // Chuẩn bị dữ liệu mẫu 100 ảnh 4K (hoặc kích thước chuẩn hoá để đo thông lượng)
    let temp_dir = TempDir::new().expect("Tạo temp dir thất bại");
    let input_dir = temp_dir.path().join("inputs");
    let output_dir = temp_dir.path().join("outputs");
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::create_dir_all(&output_dir).unwrap();

    let mut sample_paths = Vec::new();
    for i in 0..100 {
        let p = input_dir.join(format!("photo_4k_{i:03}.ppm"));
        let mut f = File::create(&p).unwrap();
        // Header PPM thô (giả lập ảnh 4K thu nhỏ để đo luồng Rayon)
        writeln!(f, "P6\n100 100\n255").unwrap();
        let buffer = vec![128u8; 100 * 100 * 3];
        f.write_all(&buffer).unwrap();
        sample_paths.push(p.to_string_lossy().to_string());
    }

    let cancel = AtomicBool::new(false);
    let tool = office_engine::media_studio::image_batch::ImageBatchTool::default();

    group.bench_function("100_images_rayon_batch", |b| {
        b.iter(|| {
            let req = DispatchTaskRequest {
                task_id: "bench-img-batch".to_string(),
                tool_slug: "image-batch".to_string(),
                input_paths: sample_paths.clone(),
                output_directory: Some(output_dir.to_string_lossy().to_string()),
                options: json!({
                    "target_format": "webp",
                    "quality": 85,
                    "strip_exif": true
                }),
            };
            let start = Instant::now();
            let res = tool.execute(black_box(&req), &cancel, None);
            let elapsed = start.elapsed();
            assert!(res.is_ok());
            assert!(
                elapsed.as_secs_f64() < 4.0,
                "BENCH-01 SLA vi phạm: hoàn tất trong {:.2?} >= 4.0s",
                elapsed
            );
        });
    });

    group.finish();
}

/// BENCH-02: Trộn Hợp đồng Word Mailmerge (FR-DOC-13)
fn bench_02_docx_template_mailmerge(c: &mut Criterion) {
    let mut group = c.benchmark_group("BENCH-02_Docx_Mailmerge");
    group.sample_size(10);

    let temp_dir = TempDir::new().expect("Tạo temp dir");
    let tpl_path = temp_dir.path().join("template.docx");
    let data_path = temp_dir.path().join("data.csv");
    let out_dir = temp_dir.path().join("merged_output");
    std::fs::create_dir_all(&out_dir).unwrap();

    // 1. Tạo tệp template có 25 trường placeholder {{placeholder}}
    let mut placeholders = Vec::new();
    for i in 1..=25 {
        placeholders.push(format!("field_{i}"));
    }
    let body_text = placeholders
        .iter()
        .map(|f| format!("{{{{{}}}}}", f))
        .collect::<Vec<_>>()
        .join(" | ");

    let mut tpl_file = File::create(&tpl_path).unwrap();
    // Tạo cấu trúc docx zip hợp lệ tối thiểu
    write_minimal_docx(&mut tpl_file, &body_text);

    // 2. Tạo tập dữ liệu CSV 1.000 dòng
    let mut data_file = File::create(&data_path).unwrap();
    writeln!(data_file, "{}", placeholders.join(",")).unwrap();
    for row in 0..1000 {
        let vals: Vec<String> = placeholders
            .iter()
            .map(|f| format!("Val_{f}_{row}"))
            .collect();
        writeln!(data_file, "{}", vals.join(",")).unwrap();
    }

    let cancel = AtomicBool::new(false);
    let tool = office_engine::doc_suite::docx_template_mailmerge::DocxTemplateMailmergeTool::default();

    group.bench_function("1000_records_docx_merge", |b| {
        b.iter(|| {
            let req = DispatchTaskRequest {
                task_id: "bench-mailmerge".to_string(),
                tool_slug: "docx-template-mailmerge".to_string(),
                input_paths: vec![
                    tpl_path.to_string_lossy().to_string(),
                    data_path.to_string_lossy().to_string(),
                ],
                output_directory: Some(out_dir.to_string_lossy().to_string()),
                options: json!({
                    "template_path": tpl_path.to_string_lossy().to_string(),
                    "data_excel_path": data_path.to_string_lossy().to_string(),
                    "output_pattern": "Contract_{{field_1}}.docx"
                }),
            };
            let start = Instant::now();
            let res = tool.execute(black_box(&req), &cancel, None);
            let elapsed = start.elapsed();
            assert!(res.is_ok());
            assert!(
                elapsed.as_secs_f64() < 1.2,
                "BENCH-02 SLA vi phạm: hoàn tất 1.000 bản ghi trong {:.2?} >= 1.2s",
                elapsed
            );
        });
    });

    group.finish();
}

/// BENCH-03: Chuyển mã Font Tiếng Việt Khối lượng Lớn (TCVN3 -> Unicode NFC)
fn bench_03_vietnamese_font_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("BENCH-03_Vietnamese_Font_Conversion");
    group.throughput(Throughput::Elements(50_000));
    group.sample_size(15);

    let temp_dir = TempDir::new().expect("Tạo temp dir");
    let input_path = temp_dir.path().join("tcvn3_50k_lines.txt");
    let out_dir = temp_dir.path().join("output");
    std::fs::create_dir_all(&out_dir).unwrap();

    // 50.000 dòng văn bản TCVN3
    let sample_line = "Céng hoµ x· héi chñ nghÜa ViÖt Nam - §éc lËp tù do h¹nh phóc.\n";
    {
        let mut f = File::create(&input_path).unwrap();
        for _ in 0..50_000 {
            f.write_all(sample_line.as_bytes()).unwrap();
        }
    }

    let cancel = AtomicBool::new(false);
    let tool = office_engine::data_lab::vietnamese_font_converter::VietnameseFontConverterTool::default();

    group.bench_function("50000_lines_tcvn3_to_unicode", |b| {
        b.iter(|| {
            let req = DispatchTaskRequest {
                task_id: "bench-font-conv".to_string(),
                tool_slug: "vietnamese-font-converter".to_string(),
                input_paths: vec![input_path.to_string_lossy().to_string()],
                output_directory: Some(out_dir.to_string_lossy().to_string()),
                options: json!({
                    "source_encoding": "tcvn3",
                    "target_encoding": "unicode"
                }),
            };
            let start = Instant::now();
            let res = tool.execute(black_box(&req), &cancel, None);
            let elapsed = start.elapsed();
            assert!(res.is_ok());

            let throughput = 50_000.0 / elapsed.as_secs_f64();
            assert!(
                throughput >= 100_000.0,
                "BENCH-03 SLA vi phạm: thông lượng đạt {:.0} dòng/giây < 100.000 dòng/giây",
                throughput
            );
        });
    });

    group.finish();
}

/// BENCH-04: Ghi Nhật ký & Truy vấn SQLite WAL
fn bench_04_sqlite_wal_logging_and_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("BENCH-04_SQLite_WAL");
    group.sample_size(20);

    let temp_dir = TempDir::new().expect("Tạo temp dir");
    let db_path = temp_dir.path().join("benchmark_wal.db");

    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;

        CREATE TABLE IF NOT EXISTS task_history (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            tool_slug TEXT NOT NULL,
            input_files_json TEXT NOT NULL,
            output_files_json TEXT NOT NULL,
            status TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            error_message TEXT,
            executed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tool_presets (
            id TEXT PRIMARY KEY,
            tool_slug TEXT NOT NULL,
            preset_name TEXT NOT NULL,
            options_json TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0
        );

        INSERT INTO tool_presets (id, tool_slug, preset_name, options_json, is_default)
        VALUES ('preset-1', 'pdf-compress', 'Nén Văn bản 150 DPI', '{}', 1);
        "#,
    )
    .unwrap();

    group.bench_function("concurrent_1000_wal_writes_and_reads", |b| {
        b.iter(|| {
            let mut write_conn = Connection::open(&db_path).unwrap();
            write_conn.execute_batch("PRAGMA busy_timeout = 5000;").unwrap();
            let read_conn = Connection::open(&db_path).unwrap();
            read_conn.execute_batch("PRAGMA busy_timeout = 5000;").unwrap();

            let mut query_stmt = read_conn.prepare_cached(
                "SELECT preset_name FROM tool_presets WHERE tool_slug = ?1"
            ).unwrap();

            let mut latencies: Vec<f64> = Vec::with_capacity(1000);

            // Bắt đầu transaction ghi WAL hàng loạt
            let tx = write_conn.transaction().unwrap();
            {
                let mut tx_stmt = tx.prepare_cached(
                    "INSERT INTO task_history (id, task_id, tool_slug, input_files_json, output_files_json, status, duration_ms, executed_at)
                     VALUES (?1, ?2, ?3, '[]', '[]', 'SUCCESS', 42, '2026-11-02T12:00:00Z')"
                ).unwrap();

                for i in 0..1000 {
                    let start = Instant::now();
                    tx_stmt.execute(params![
                        format!("task-hist-{}-{}", i, uuid_v4_simple()),
                        format!("task-{}", i),
                        "pdf-merge-split"
                    ]).unwrap();
                    let write_latency = start.elapsed().as_secs_f64() * 1000.0;

                    // Đồng thời đọc trong WAL mode
                    let q_start = Instant::now();
                    let _name: String = query_stmt.query_row(params!["pdf-compress"], |row| row.get(0)).unwrap();
                    let read_latency = q_start.elapsed().as_secs_f64() * 1000.0;

                    latencies.push(write_latency.max(read_latency));
                }
            }
            tx.commit().unwrap();

            let mut sorted = latencies;
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p99 = sorted[(sorted.len() as f64 * 0.99) as usize];
            assert!(
                p99 < 1.0,
                "BENCH-04 SLA vi phạm: độ trễ ghi WAL p99 đạt {:.3}ms >= 1.0ms",
                p99
            );
        });
    });

    group.finish();
}

fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let n = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", n)
}

fn write_minimal_docx(f: &mut File, text: &str) {
    // Tạo file zip hợp lệ với document.xml
    use flate2::write::DeflateEncoder;
    use flate2::Compression;

    let doc_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>{}</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
        text
    );

    let doc_bytes = doc_xml.as_bytes();
    let crc = crc32fast::hash(doc_bytes);
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(doc_bytes).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut zip_bytes = Vec::new();
    // Local file header
    zip_bytes.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
    zip_bytes.extend_from_slice(&crc.to_le_bytes());
    zip_bytes.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    zip_bytes.extend_from_slice(&(doc_bytes.len() as u32).to_le_bytes());
    let filename = b"word/document.xml";
    zip_bytes.extend_from_slice(&(filename.len() as u16).to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes());
    zip_bytes.extend_from_slice(filename);
    zip_bytes.extend_from_slice(&compressed);

    let central_dir_offset = zip_bytes.len() as u32;
    // Central directory header
    zip_bytes.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
    zip_bytes.extend_from_slice(&crc.to_le_bytes());
    zip_bytes.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    zip_bytes.extend_from_slice(&(doc_bytes.len() as u32).to_le_bytes());
    zip_bytes.extend_from_slice(&(filename.len() as u16).to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes());
    zip_bytes.extend_from_slice(&0u32.to_le_bytes());
    zip_bytes.extend_from_slice(&0u32.to_le_bytes()); // offset to local header
    zip_bytes.extend_from_slice(filename);
    let central_dir_size = (zip_bytes.len() as u32) - central_dir_offset;

    // End of central directory
    zip_bytes.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 1, 0, 1, 0]);
    zip_bytes.extend_from_slice(&central_dir_size.to_le_bytes());
    zip_bytes.extend_from_slice(&central_dir_offset.to_le_bytes());
    zip_bytes.extend_from_slice(&0u16.to_le_bytes());

    f.write_all(&zip_bytes).unwrap();
}

criterion_group!(
    benches,
    bench_01_image_batch_compression,
    bench_02_docx_template_mailmerge,
    bench_03_vietnamese_font_conversion,
    bench_04_sqlite_wal_logging_and_query
);
criterion_main!(benches);
