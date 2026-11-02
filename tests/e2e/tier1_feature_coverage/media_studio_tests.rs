use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::json;
use office_shared::DispatchTaskRequest;
use crate::harness::TestHarness;

macro_rules! define_media_studio_tier1_tests {
    ($slug:ident, $slug_str:expr, $happy_opts:expr, $second_opts:expr, $matrix_opts:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $slug _happy_path>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => {
                        vec![harness.create_valid_wav(concat!($slug_str, "_happy.wav")).to_string_lossy().to_string()]
                    }
                    "video-to-gif-webp" => {
                        vec![harness.create_valid_mp4(concat!($slug_str, "_happy.mp4")).to_string_lossy().to_string()]
                    }
                    "image-exif-inspector" => {
                        vec![harness.create_valid_png(concat!($slug_str, "_happy.png")).to_string_lossy().to_string()]
                    }
                    _ => {
                        let img = harness.create_valid_bmp(concat!($slug_str, "_happy.bmp"), 64, 64);
                        vec![img.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-happy", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Happy path must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _secondary_mode>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => {
                        vec![harness.create_valid_wav(concat!($slug_str, "_second.wav")).to_string_lossy().to_string()]
                    }
                    "video-to-gif-webp" => {
                        vec![harness.create_valid_mp4(concat!($slug_str, "_second.mp4")).to_string_lossy().to_string()]
                    }
                    "image-exif-inspector" => {
                        vec![harness.create_valid_png(concat!($slug_str, "_second.png")).to_string_lossy().to_string()]
                    }
                    _ => {
                        let img = harness.create_valid_bmp(concat!($slug_str, "_second.bmp"), 48, 48);
                        vec![img.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-second", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $second_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Secondary mode must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _batch_processing>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => {
                        let w1 = harness.create_valid_wav(concat!($slug_str, "_b1.wav"));
                        let w2 = harness.create_valid_wav(concat!($slug_str, "_b2.wav"));
                        vec![w1.to_string_lossy().to_string(), w2.to_string_lossy().to_string()]
                    }
                    "video-to-gif-webp" => {
                        let v1 = harness.create_valid_mp4(concat!($slug_str, "_b1.mp4"));
                        let v2 = harness.create_valid_mp4(concat!($slug_str, "_b2.mp4"));
                        vec![v1.to_string_lossy().to_string(), v2.to_string_lossy().to_string()]
                    }
                    "image-exif-inspector" => {
                        let p1 = harness.create_valid_png(concat!($slug_str, "_b1.png"));
                        let p2 = harness.create_valid_png(concat!($slug_str, "_b2.png"));
                        vec![p1.to_string_lossy().to_string(), p2.to_string_lossy().to_string()]
                    }
                    _ => {
                        let img1 = harness.create_valid_bmp(concat!($slug_str, "_b1.bmp"), 32, 32);
                        let img2 = harness.create_valid_bmp(concat!($slug_str, "_b2.bmp"), 32, 32);
                        vec![img1.to_string_lossy().to_string(), img2.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-batch", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Batch must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _options_matrix>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => {
                        vec![harness.create_valid_wav(concat!($slug_str, "_opt.wav")).to_string_lossy().to_string()]
                    }
                    "video-to-gif-webp" => {
                        vec![harness.create_valid_mp4(concat!($slug_str, "_opt.mp4")).to_string_lossy().to_string()]
                    }
                    "image-exif-inspector" => {
                        vec![harness.create_valid_png(concat!($slug_str, "_opt.png")).to_string_lossy().to_string()]
                    }
                    _ => {
                        let img = harness.create_valid_bmp(concat!($slug_str, "_opt.bmp"), 32, 32);
                        vec![img.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-opt", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $matrix_opts,
                };
                let cancel = AtomicBool::new(false);
                if let Some(tool) = harness.get_tool($slug_str) {
                    let res = tool.execute(&req, &cancel, None).expect("Tier 1 Options matrix must succeed");
                    assert!(res.status == "SUCCESS" || res.status == "PARTIAL_SUCCESS");
                }
            }

            #[test]
            fn [<test_ $slug _progress_callback>]() {
                let harness = TestHarness::new();
                let input_paths = match $slug_str {
                    "audio-silence-trimmer" => {
                        vec![harness.create_valid_wav(concat!($slug_str, "_prog.wav")).to_string_lossy().to_string()]
                    }
                    "video-to-gif-webp" => {
                        vec![harness.create_valid_mp4(concat!($slug_str, "_prog.mp4")).to_string_lossy().to_string()]
                    }
                    "image-exif-inspector" => {
                        vec![harness.create_valid_png(concat!($slug_str, "_prog.png")).to_string_lossy().to_string()]
                    }
                    _ => {
                        let img = harness.create_valid_bmp(concat!($slug_str, "_prog.bmp"), 32, 32);
                        vec![img.to_string_lossy().to_string()]
                    }
                };
                let req = DispatchTaskRequest {
                    task_id: format!("{}-prog", $slug_str),
                    tool_slug: $slug_str.to_string(),
                    input_paths,
                    output_directory: Some(harness.temp_dir().to_string_lossy().to_string()),
                    options: $happy_opts,
                };
                let cancel = AtomicBool::new(false);
                let progress_called = Arc::new(AtomicBool::new(false));
                let p_clone = Arc::clone(&progress_called);
                let cb = move |p: office_shared::TaskProgress| {
                    p_clone.store(true, Ordering::Relaxed);
                    assert!(p.percentage >= 0.0 && p.percentage <= 100.0);
                };

                if let Some(tool) = harness.get_tool($slug_str) {
                    let _ = tool.execute(&req, &cancel, Some(&cb));
                    assert!(progress_called.load(Ordering::Relaxed), "Progress callback must be invoked");
                }
            }
        }
    };
}

// 20. image-batch (FR-MED-01)
define_media_studio_tier1_tests!(
    image_batch,
    "image-batch",
    json!({ "target_format": "png", "quality": 85, "strip_exif": true }),
    json!({ "target_format": "jpeg", "quality": 75, "max_width": 800 }),
    json!({ "target_format": "bmp", "strip_exif": false })
);

// 21. photo-id-crop (FR-MED-02)
define_media_studio_tier1_tests!(
    photo_id_crop,
    "photo-id-crop",
    json!({ "standard_size": "4x6", "dpi": 300, "background_color": "white" }),
    json!({ "standard_size": "3x4", "dpi": 300, "background_color": "blue" }),
    json!({ "standard_size": "passport_vn", "grid_layout": "10x15_sheet", "cut_marks": true })
);

// 22. image-bg-redact (FR-MED-03)
define_media_studio_tier1_tests!(
    image_bg_redact,
    "image-bg-redact",
    json!({ "mode": "blackout", "regions": [[10, 10, 50, 20]] }),
    json!({ "mode": "pixelate", "pixel_size": 8, "regions": [[5, 5, 40, 30]] }),
    json!({ "mode": "gaussian_blur", "sigma": 5.0, "auto_detect_pii": true })
);

// 23. image-exif-inspector (FR-MED-04)
define_media_studio_tier1_tests!(
    image_exif_inspector,
    "image-exif-inspector",
    json!({ "action": "strip_all", "preserve_color_profile": true }),
    json!({ "action": "inspect_only" }),
    json!({ "action": "strip_gps_only", "wipe_camera_serial": true })
);

// 24. watermark-batch-studio (FR-MED-05)
define_media_studio_tier1_tests!(
    watermark_batch_studio,
    "watermark-batch-studio",
    json!({ "text": "OFFICE UTILITY VN", "position": "bottom-right", "opacity": 0.4 }),
    json!({ "type": "tiled_diagonal", "text": "CONFIDENTIAL", "opacity": 0.2 }),
    json!({ "position": "center", "font_size": 24, "color_hex": "#FFFFFF" })
);

// 25. screenshot-beautifier (FR-MED-06)
define_media_studio_tier1_tests!(
    screenshot_beautifier,
    "screenshot-beautifier",
    json!({ "window_frame": "macos_dark", "shadow_radius": 20, "padding": 40 }),
    json!({ "window_frame": "windows11_light", "shadow_radius": 15 }),
    json!({ "background_gradient": "sunset", "corner_radius": 12, "inset_shadow": true })
);

// 26. doc-scanner-perspective (FR-MED-07)
define_media_studio_tier1_tests!(
    doc_scanner_perspective,
    "doc-scanner-perspective",
    json!({ "auto_corners": true, "filter": "black_and_white_enhance" }),
    json!({ "corners": [[0, 0], [100, 10], [90, 100], [5, 95]], "target_ratio": "A4" }),
    json!({ "contrast_boost": 1.5, "dewarp": false, "color_mode": "grayscale" })
);

// 27. audio-silence-trimmer (FR-MED-08)
define_media_studio_tier1_tests!(
    audio_silence_trimmer,
    "audio-silence-trimmer",
    json!({ "silence_threshold_db": -40.0, "min_silence_duration_ms": 500 }),
    json!({ "silence_threshold_db": -35.0, "crossfade_ms": 50 }),
    json!({ "silence_threshold_db": -50.0, "normalize_output": true })
);

// 28. video-to-gif-webp (FR-MED-09)
define_media_studio_tier1_tests!(
    video_to_gif_webp,
    "video-to-gif-webp",
    json!({ "output_format": "gif", "fps": 15, "width": 480, "quantizer": "neuquant" }),
    json!({ "output_format": "webp", "fps": 24, "quality": 80 }),
    json!({ "output_format": "gif", "speed_multiplier": 1.5, "loop_count": 0 })
);
