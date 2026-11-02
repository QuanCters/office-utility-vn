pub mod registry;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use office_engine::OfficeTool;
use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, TaskProgress,
};
use tempfile::TempDir;

pub struct TestHarness {
    temp_dir: TempDir,
}

impl TestHarness {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temporary directory for test harness"),
        }
    }

    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn get_tool(&self, slug: &str) -> Option<Box<dyn OfficeTool>> {
        registry::resolve_tool(slug)
    }

    /// Tạo tệp PDF hợp lệ với số trang chỉ định
    pub fn create_valid_pdf(&self, filename: &str, page_count: usize) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let mut pdf_content = String::new();
        pdf_content.push_str("%PDF-1.4\n");
        pdf_content.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages root
        let mut kids = String::new();
        for i in 0..page_count {
            kids.push_str(&format!("{} 0 R ", 3 + i));
        }
        pdf_content.push_str(&format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.trim(),
            page_count
        ));

        // Page objects
        for i in 0..page_count {
            pdf_content.push_str(&format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n",
                3 + i
            ));
        }

        // Xref & trailer
        pdf_content.push_str("xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n12345\n%%EOF\n");

        fs::write(&path, pdf_content.as_bytes()).expect("write valid pdf");
        path
    }

    /// Tạo tệp nhị phân có header bị hỏng (magic bytes không hợp lệ)
    pub fn create_corrupt_file(&self, filename: &str, corrupt_header: &[u8]) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let mut data = Vec::from(corrupt_header);
        data.extend_from_slice(b"\x00\xFF\xAA\x55_CORRUPTED_PAYLOAD_BODY_PADDING_");
        fs::write(&path, data).expect("write corrupt file");
        path
    }

    /// Tạo tệp ảnh BMP hợp lệ (24-bit RGB)
    pub fn create_valid_bmp(&self, filename: &str, width: u32, height: u32) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let row_size = ((24 * width as usize + 31) / 32) * 4;
        let pixel_array_size = row_size * height as usize;
        let file_size = 54 + pixel_array_size;

        let mut data = Vec::with_capacity(file_size);
        // Header (14 bytes)
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&(file_size as u32).to_le_bytes());
        data.extend_from_slice(&[0, 0, 0, 0]); // Reserved
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset to pixel array

        // DIB Header (40 bytes - BITMAPINFOHEADER)
        data.extend_from_slice(&40u32.to_le_bytes());
        data.extend_from_slice(&(width as i32).to_le_bytes());
        data.extend_from_slice(&(height as i32).to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&(pixel_array_size as u32).to_le_bytes());
        data.extend_from_slice(&2835u32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&2835u32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors in palette
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Pixels (RGB)
        for _ in 0..height {
            for _ in 0..width {
                data.extend_from_slice(&[180, 100, 50]); // BGR
            }
            let padding = row_size - (width as usize * 3);
            data.extend(std::iter::repeat(0).take(padding));
        }

        fs::write(&path, data).expect("write valid bmp");
        path
    }

    /// Tạo tệp ảnh PNG với magic bytes chuẩn
    pub fn create_valid_png(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        // PNG tối thiểu 1x1 pixel
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        // IHDR chunk: 1x1, 8-bit RGB
        png.extend_from_slice(&13u32.to_be_bytes()); // Length 13
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&1u32.to_be_bytes()); // Width 1
        png.extend_from_slice(&1u32.to_be_bytes()); // Height 1
        png.push(8); // Bit depth
        png.push(2); // Color type RGB
        png.push(0); // Compression
        png.push(0); // Filter
        png.push(0); // Interlace
        png.extend_from_slice(&[0x4A, 0xD4, 0x18, 0xE9]); // IHDR CRC

        // IEND chunk
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]); // IEND CRC

        fs::write(&path, png).expect("write valid png");
        path
    }

    /// Tạo tệp văn bản UTF-8
    pub fn create_valid_text_file(&self, filename: &str, content: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        fs::write(&path, content.as_bytes()).expect("write valid text file");
        path
    }

    /// Tạo tệp CSV bảng tính
    pub fn create_valid_csv(&self, filename: &str, rows: &[&[&str]]) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let mut content = String::new();
        for row in rows {
            content.push_str(&row.join(","));
            content.push('\n');
        }
        fs::write(&path, content.as_bytes()).expect("write valid csv");
        path
    }

    /// Tạo tệp XML Hóa đơn Điện tử theo Nghị định 123 / Thông tư 78
    pub fn create_valid_xml_invoice(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<HDon>
    <DLHDon Id="HD123456">
        <TTChung>
            <PBan>2.0.0</PBan>
            <THDon>Hóa đơn giá trị gia tăng</THDon>
            <KHMSHDon>1</KHMSHDon>
            <KHHDon>C26TAA</KHHDon>
            <SHDon>0001234</SHDon>
            <NLap>2026-11-02</NLap>
            <DVTTe>VND</DVTTe>
            <TGia>1.0</TGia>
            <HTTToan>TM/CK</HTTToan>
        </TTChung>
        <NDHDon>
            <NBan>
                <Ten>CÔNG TY TNHH CÔNG NGHỆ TIỆN ÍCH VIỆT NAM</Ten>
                <MST>0101234567</MST>
                <DChi>Số 10 Phố Tràng Tiền, Phường Tràng Tiền, Quận Hoàn Kiếm, Hà Nội</DChi>
            </NBan>
            <NMua>
                <Ten>NGUYỄN VĂN AN</Ten>
                <MST>8001234567</MST>
                <DChi>123 Đường Nguyễn Huệ, Phường Bến Nghé, Quận 1, TP Hồ Chí Minh</DChi>
            </NMua>
            <DSHHang>
                <HHDVu>
                    <TCat>1</TCat>
                    <STT>1</STT>
                    <THang>Bản quyền Phần mềm Office Utility VN Enterprise</THang>
                    <DVTinh>Gói</DVTinh>
                    <SLuong>1</SLuong>
                    <DGia>15000000</DGia>
                    <Tien>15000000</Tien>
                    <TSuat>10%</TSuat>
                </HHDVu>
            </DSHHang>
            <TToan>
                <TgTCThue>15000000</TgTCThue>
                <TgTThue>1500000</TgTThue>
                <TgTTTBSo>16500000</TgTTTBSo>
                <TgTTTBChu>Mười sáu triệu năm trăm nghìn đồng chẵn</TgTTTBChu>
            </TToan>
        </NDHDon>
    </DLHDon>
</HDon>"#;
        fs::write(&path, xml.as_bytes()).expect("write xml invoice");
        path
    }

    /// Tạo tệp WAV hợp lệ với âm thanh mẫu 16-bit PCM 16kHz
    pub fn create_valid_wav(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let sample_rate = 16000u32;
        let tone_samples = 1600; // 100ms
        let silence_samples = 8000; // 500ms
        let mut samples = Vec::with_capacity(tone_samples * 2 + silence_samples);

        for i in 0..tone_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            samples.push((val * 16000.0) as i16);
        }
        for _ in 0..silence_samples {
            samples.push(0i16);
        }
        for i in 0..tone_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (t * 880.0 * 2.0 * std::f32::consts::PI).sin();
            samples.push((val * 16000.0) as i16);
        }

        let data_len = samples.len() * 2;
        let mut wav = Vec::with_capacity(44 + data_len);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }

        fs::write(&path, wav).expect("write valid wav");
        path
    }

    /// Tạo tệp MP4 hợp lệ với container `ftyp`
    pub fn create_valid_mp4(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let mut mp4 = Vec::new();
        mp4.extend_from_slice(&20u32.to_be_bytes());
        mp4.extend_from_slice(b"ftyp");
        mp4.extend_from_slice(b"isom");
        mp4.extend_from_slice(&[0, 0, 2, 0]);
        mp4.extend_from_slice(b"isom");
        mp4.extend_from_slice(b"\x00\x00\x00\x08free");
        fs::write(&path, mp4).expect("write valid mp4");
        path
    }

    /// Tạo tệp Word .docx hợp lệ
    pub fn create_valid_docx(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body><w:p><w:r><w:t>Kính gửi {{ho_ten}}, Mã: {{ma_nv}}</w:t></w:r></w:p></w:body>
</w:document>"#;
        let name = "word/document.xml";
        let mut zip = Vec::new();
        zip.extend_from_slice(b"PK\x03\x04");
        zip.extend_from_slice(&20u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes()); // method 0 (stored)
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u32.to_le_bytes());
        zip.extend_from_slice(&(doc_xml.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(doc_xml.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(name.as_bytes());
        zip.extend_from_slice(doc_xml.as_bytes());

        fs::write(&path, zip).expect("write valid docx");
        path
    }

    /// Tạo tệp Excel .xlsx hợp lệ
    pub fn create_valid_xlsx(&self, filename: &str) -> PathBuf {
        let path = self.temp_dir.path().join(filename);
        let sheet_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <sheetProtection sheet="1" password="123"/>
    <sheetData/>
</worksheet>"#;
        let name = "xl/worksheets/sheet1.xml";
        let mut zip = Vec::new();
        zip.extend_from_slice(b"PK\x03\x04");
        zip.extend_from_slice(&20u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(&0u32.to_le_bytes());
        zip.extend_from_slice(&(sheet_xml.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(sheet_xml.len() as u32).to_le_bytes());
        zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes());
        zip.extend_from_slice(name.as_bytes());
        zip.extend_from_slice(sheet_xml.as_bytes());

        fs::write(&path, zip).expect("write valid xlsx");
        path
    }

    /// Thực thi công cụ qua `OfficeTool::execute`
    pub fn execute(
        &self,
        slug: &str,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Option<Result<DispatchTaskResponse, CommandError>> {
        self.get_tool(slug)
            .map(|tool| tool.execute(request, cancel_flag, on_progress))
    }

    /// Kiểm tra SLA hủy tác vụ: phải trả về `CommandError::TaskCancelled` trong thời gian < 100ms
    pub fn assert_cancellation_under_100ms<F>(&self, op: F)
    where
        F: FnOnce(&AtomicBool) -> Result<DispatchTaskResponse, CommandError>,
    {
        let cancel = AtomicBool::new(true);
        let start = Instant::now();
        let res = op(&cancel);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "Vi phạm SLA hủy tác vụ (NFR-REL-02): Mất {}ms (Yêu cầu < 100ms)",
            elapsed.as_millis()
        );

        match res {
            Err(CommandError::TaskCancelled) => {
                // Đạt chuẩn SLA hủy tác vụ
            }
            other => {
                panic!("Kỳ vọng CommandError::TaskCancelled khi cancel_flag=true, nhưng nhận được: {:?}", other);
            }
        }
    }

    /// Kiểm tra xác thực Magic Bytes / Chữ ký tệp: tệp hỏng phải bị từ chối
    pub fn assert_invalid_signature(
        &self,
        slug: &str,
        corrupt_input: &Path,
        options: serde_json::Value,
    ) {
        if let Some(tool) = self.get_tool(slug) {
            let opts = if slug == "vietnamese-number-to-words" {
                serde_json::json!({ "number": "invalid_number_abc" })
            } else {
                options
            };
            let req = DispatchTaskRequest {
                task_id: format!("test-bad-sig-{}", slug),
                tool_slug: slug.to_string(),
                input_paths: vec![corrupt_input.to_string_lossy().to_string()],
                output_directory: Some(self.temp_dir.path().to_string_lossy().to_string()),
                options: opts,
            };
            let cancel = AtomicBool::new(false);
            let res = tool.execute(&req, &cancel, None);
            match res {
                Err(CommandError::InvalidFileSignature { .. })
                | Err(CommandError::ValidationFailed { .. })
                | Err(CommandError::ProcessExecutionError { .. }) => {
                    // Thành công chặn tệp có magic bytes không hợp lệ
                }
                Ok(resp) => {
                    // Nếu công cụ cho phép tệp tiếp tục nhưng ghi nhận lỗi trong failed_items
                    assert!(
                        !resp.failed_items.is_empty() || resp.status == "FAILED",
                        "Công cụ '{}' không được chấp nhận tệp có magic bytes bị hỏng thành công!",
                        slug
                    );
                }
                Err(other) => {
                    // Các lỗi khác vẫn chấp nhận được miễn là không sập tiến trình
                    eprintln!("Công cụ '{}' từ chối tệp với lỗi: {:?}", slug, other);
                }
            }
        }
    }
}
