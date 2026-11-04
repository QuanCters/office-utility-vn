use std::fs;
use std::path::Path;

const EXPECTED_TOKENS: &[&str] = &[
    "khong", "mot", "hai", "ba", "bon", "nam", "sau", "bay", "tam", "chin",
    "mot_sau", "tu", "lam", "muoi", "muoi_chuc", "tram", "nghin", "ngan", "trieu", "ty",
    "le", "linh", "am", "va", "dong", "chan", "xu", "do_la_my", "euro", "cents",
    "mot_nam", "tu_nam", "chuc", "phay", "cham"
];

#[test]
fn test_audio_soundpack_35_files_exist_and_valid() {
    let base_dir = Path::new("public/audio/vn_numbers");
    assert!(base_dir.exists(), "Thư mục soundpack public/audio/vn_numbers phải tồn tại");

    for token in EXPECTED_TOKENS {
        let file_path = base_dir.join(format!("{token}.mp3"));
        assert!(file_path.exists(), "Mẫu âm '{token}.mp3' phải tồn tại");

        let metadata = fs::metadata(&file_path).unwrap_or_else(|_| panic!("Không thể đọc metadata của {:?}", file_path));
        assert!(metadata.len() > 500, "File {:?} quá nhỏ: {} bytes", file_path, metadata.len());

        let bytes = fs::read(&file_path).unwrap_or_else(|_| panic!("Không thể đọc bytes của {:?}", file_path));
        // Kiểm tra chữ ký MP3: ID3 tag hoặc MP3 frame sync 0xFF
        let is_valid_mp3 = (bytes.len() >= 3 && &bytes[0..3] == b"ID3")
            || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0);
        assert!(is_valid_mp3, "File {:?} không phải định dạng MP3 hợp lệ", file_path);
    }
}
