use std::sync::atomic::AtomicBool;
use serde_json::json;
use office_shared::{CommandError, DispatchTaskRequest};
use crate::harness::TestHarness;

macro_rules! define_media_studio_tier2_boundary_tests {
    ($slug:ident, $slug_str:expr, $valid_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _invalid_magic_bytes>]() {
                let harness = TestHarness::new();
                let bad = harness.create_corrupt_file(concat!($slug_str, "_bad.img"), b"NOT_VALID_IMAGE_MAGIC_FF_D8_DEADBEEF");
                harness.assert_invalid_signature($slug_str, &bad, $valid_opts);
            }

            #[test]
            fn [<test_ $slug _empty_inputs>]() {
                let harness = TestHarness::new();
                if let Some(tool) = harness.get_tool($slug_str) {
                    let req = DispatchTaskRequest {
                        task_id: format!("{}-empty", $slug_str),
                        tool_slug: $slug_str.to_string(),
                        input_paths: vec![],
                        output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                        options: $valid_opts,
                    };
                    let cancel = AtomicBool::new(false);
                    let res = tool.execute(&req, &cancel, None);
                    match res {
                        Err(CommandError::ValidationFailed { .. }) => {}
                        other => {
                            panic!("Kỳ vọng ValidationFailed khi input_paths trống, nhưng nhận: {:?}", other);
                        }
                    }
                }
            }

            #[test]
            fn [<test_ $slug _immediate_cancellation>]() {
                let harness = TestHarness::new();
                let img = harness.create_valid_bmp(concat!($slug_str, "_cancel.bmp"), 32, 32);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-imm", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![img.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $valid_opts,
                };
                if let Some(tool) = harness.get_tool($slug_str) {
                    harness.assert_cancellation_under_100ms(|cancel| {
                        tool.execute(&req, cancel, None)
                    });
                }
            }

            #[test]
            fn [<test_ $slug _midflight_cancellation>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => vec![
                        harness.create_valid_wav(concat!($slug_str, "_mid1.wav")).to_string_lossy().to_string(),
                        harness.create_valid_wav(concat!($slug_str, "_mid2.wav")).to_string_lossy().to_string(),
                    ],
                    "video-to-gif-webp" => vec![
                        harness.create_valid_mp4(concat!($slug_str, "_mid1.mp4")).to_string_lossy().to_string(),
                        harness.create_valid_mp4(concat!($slug_str, "_mid2.mp4")).to_string_lossy().to_string(),
                    ],
                    "image-exif-inspector" => vec![
                        harness.create_valid_png(concat!($slug_str, "_mid1.png")).to_string_lossy().to_string(),
                        harness.create_valid_png(concat!($slug_str, "_mid2.png")).to_string_lossy().to_string(),
                    ],
                    _ => vec![
                        harness.create_valid_bmp(concat!($slug_str, "_mid1.bmp"), 64, 64).to_string_lossy().to_string(),
                        harness.create_valid_bmp(concat!($slug_str, "_mid2.bmp"), 64, 64).to_string_lossy().to_string(),
                    ],
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-cancel-mid", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $valid_opts,
                };
                if let Some(tool) = harness.get_tool($slug_str) {
                    let cancel = AtomicBool::new(false);
                    let start = std::time::Instant::now();
                    let cb = |_: office_shared::TaskProgress| {
                        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    };
                    let res = tool.execute(&req, &cancel, Some(&cb));
                    let elapsed = start.elapsed();
                    assert!(elapsed.as_millis() < 500, "Cooperative cancellation must react rapidly");
                    assert!(
                        matches!(res, Err(CommandError::TaskCancelled)),
                        "Kỳ vọng CommandError::TaskCancelled khi bị hủy giữa chừng, nhận được: {:?}",
                        res
                    );
                }
            }

            #[test]
            fn [<test_ $slug _extreme_boundary>]() {
                let harness = TestHarness::new();
                let img = harness.create_valid_bmp(concat!($slug_str, "_extreme.bmp"), 16, 16);
                let req = DispatchTaskRequest {
                    task_id: format!("{}-extreme", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths: vec![img.to_string_lossy().to_string()],
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: json!({
                        "quality": 100,
                        "max_width": 16384,
                        "text": "BẢN QUYỀN HÌNH ẢNH VIỆT NAM © 2026 🇻🇳",
                        "opacity": 1.0,
                        "dpi": 600
                    }),
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let _ = tool.execute(&req, &cancel, None);
                }
            }
        }
    };
}

// 9 tools trong Media Studio
define_media_studio_tier2_boundary_tests!(image_batch, "image-batch", json!({ "target_format": "png" }));
define_media_studio_tier2_boundary_tests!(photo_id_crop, "photo-id-crop", json!({ "standard_size": "4x6" }));
define_media_studio_tier2_boundary_tests!(image_bg_redact, "image-bg-redact", json!({ "mode": "blackout" }));
define_media_studio_tier2_boundary_tests!(image_exif_inspector, "image-exif-inspector", json!({ "action": "strip_all" }));
define_media_studio_tier2_boundary_tests!(watermark_batch_studio, "watermark-batch-studio", json!({ "text": "VN" }));
define_media_studio_tier2_boundary_tests!(screenshot_beautifier, "screenshot-beautifier", json!({ "window_frame": "macos_dark" }));
define_media_studio_tier2_boundary_tests!(doc_scanner_perspective, "doc-scanner-perspective", json!({ "auto_corners": true }));
define_media_studio_tier2_boundary_tests!(audio_silence_trimmer, "audio-silence-trimmer", json!({ "silence_threshold_db": -40.0 }));
define_media_studio_tier2_boundary_tests!(video_to_gif_webp, "video-to-gif-webp", json!({ "output_format": "gif" }));
