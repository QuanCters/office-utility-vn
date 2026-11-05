use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use office_shared::{
    CommandError, DispatchTaskRequest, DispatchTaskResponse, FileFailureDetail, TaskProgress,
};
use serde::{Deserialize, Serialize};

use crate::OfficeTool;

/// Tùy chọn cấu hình cắt & xử lý âm thanh (khoảng lặng, mốc thời gian thủ công, khuếch đại volume)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrimOptions {
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold_db: f32, // Ngưỡng decibel xem là im lặng (ví dụ -40.0 dB)
    #[serde(default = "default_min_silence_duration_ms")]
    pub min_silence_duration_ms: u32, // Thời lượng khoảng lặng tối thiểu cần cắt (ms)
    // Chế độ cắt theo mốc thời gian thủ công (Manual Trim)
    #[serde(default)]
    pub manual_trim_enabled: bool,
    #[serde(default)]
    pub start_time_ms: Option<u32>,
    #[serde(default)]
    pub end_time_ms: Option<u32>,
    // Khuếch đại âm lượng (Volume Booster)
    #[serde(default)]
    pub volume_boost_db: Option<f32>,
    #[serde(default)]
    pub enable_limiter: bool,
}

fn default_silence_threshold() -> f32 {
    -40.0
}

fn default_min_silence_duration_ms() -> u32 {
    300
}

pub struct AudioSilenceTrimmerTool;

impl Default for AudioSilenceTrimmerTool {
    fn default() -> Self {
        Self
    }
}

/// Dữ liệu luồng âm thanh PCM 16-bit
#[derive(Debug, Clone)]
pub struct PcmAudio {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>, // Interleaved samples
}

/// Đọc và giải mã luồng âm thanh (WAV PCM hoặc MP3 qua Symphonia pure-Rust)
pub fn decode_audio(bytes: &[u8]) -> Result<PcmAudio, CommandError> {
    if bytes.len() < 4 {
        return Err(CommandError::InvalidFileSignature {
            expected: "WAV (RIFF) hoặc MP3 (ID3)".to_string(),
            actual: "Tệp quá nhỏ".to_string(),
        });
    }

    if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WAVE" {
        decode_wav(bytes)
    } else if bytes.starts_with(b"ID3") || bytes.starts_with(&[0xFF, 0xFB]) || bytes.starts_with(&[0xFF, 0xF3]) {
        decode_mp3(bytes)
    } else {
        Err(CommandError::InvalidFileSignature {
            expected: "WAV (RIFF) hoặc MP3 (ID3)".to_string(),
            actual: format!("{:02X?}", &bytes[0..bytes.len().min(4)]),
        })
    }
}

/// Giải mã MP3 thuần Rust 100% qua puremp3 crate
pub fn decode_mp3(bytes: &[u8]) -> Result<PcmAudio, CommandError> {
    let (header, sample_iter) = puremp3::read_mp3(bytes).map_err(|e| CommandError::ValidationFailed {
        reason: format!("Lỗi giải mã MP3: {e:?}"),
    })?;

    let is_mono = matches!(header.channels, puremp3::Channels::Mono);
    let channels = if is_mono { 1 } else { 2 };
    let sample_rate = header.sample_rate.hz();

    let mut samples = Vec::new();
    for (l, r) in sample_iter {
        let left_i16 = (l.clamp(-1.0, 1.0) * 32767.0) as i16;
        samples.push(left_i16);
        if !is_mono {
            let right_i16 = (r.clamp(-1.0, 1.0) * 32767.0) as i16;
            samples.push(right_i16);
        }
    }

    Ok(PcmAudio {
        channels,
        sample_rate,
        samples,
    })
}

/// Đọc và xác thực tệp WAV PCM 16-bit
pub fn decode_wav(bytes: &[u8]) -> Result<PcmAudio, CommandError> {
    if bytes.len() < 44 {
        return Err(CommandError::InvalidFileSignature {
            expected: "WAV (RIFF....WAVE)".to_string(),
            actual: "Tệp quá nhỏ để là WAV hợp lệ".to_string(),
        });
    }

    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(CommandError::InvalidFileSignature {
            expected: "WAV (RIFF....WAVE)".to_string(),
            actual: format!("{:02X?}", &bytes[0..bytes.len().min(4)]),
        });
    }

    let mut cursor = 12;
    let mut channels = 1u16;
    let mut sample_rate = 44100u32;
    let mut bits_per_sample = 16u16;
    let mut pcm_samples = Vec::new();

    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]) as usize;

        let data_start = cursor + 8;
        let data_end = (data_start + chunk_size).min(bytes.len());

        match chunk_id {
            b"fmt " => {
                if chunk_size >= 16 && data_start + 16 <= bytes.len() {
                    let audio_format = u16::from_le_bytes([bytes[data_start], bytes[data_start + 1]]);
                    if audio_format != 1 {
                        // Không phải PCM nguyên bản
                        return Err(CommandError::ValidationFailed {
                            reason: format!("Chỉ hỗ trợ định dạng WAV PCM không nén (format {audio_format})"),
                        });
                    }
                    channels = u16::from_le_bytes([bytes[data_start + 2], bytes[data_start + 3]]);
                    sample_rate = u32::from_le_bytes([
                        bytes[data_start + 4],
                        bytes[data_start + 5],
                        bytes[data_start + 6],
                        bytes[data_start + 7],
                    ]);
                    bits_per_sample = u16::from_le_bytes([bytes[data_start + 14], bytes[data_start + 15]]);
                }
            }
            b"data" => {
                if bits_per_sample == 16 {
                    let num_samples = (data_end - data_start) / 2;
                    pcm_samples.reserve(num_samples);
                    for i in 0..num_samples {
                        let offset = data_start + i * 2;
                        if offset + 2 <= bytes.len() {
                            let s = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                            pcm_samples.push(s);
                        }
                    }
                } else if bits_per_sample == 8 {
                    let num_samples = data_end - data_start;
                    pcm_samples.reserve(num_samples);
                    for i in 0..num_samples {
                        let offset = data_start + i;
                        if offset < bytes.len() {
                            // 8-bit WAV là unsigned (0..255) với 128 là điểm 0
                            let s8 = bytes[offset] as i32 - 128;
                            pcm_samples.push((s8 * 256) as i16);
                        }
                    }
                }
            }
            _ => {}
        }

        cursor = data_end;
        if chunk_size % 2 == 1 {
            cursor += 1; // Word alignment padding
        }
    }

    Ok(PcmAudio {
        channels: channels.max(1),
        sample_rate: sample_rate.max(8000),
        samples: pcm_samples,
    })
}

/// Mã hóa mảng mẫu PCM 16-bit thành tệp WAV chuẩn
pub fn encode_wav(audio: &PcmAudio) -> Vec<u8> {
    let mut out = Vec::new();
    let channels = audio.channels.max(1);
    let sample_rate = audio.sample_rate.max(8000);
    let bits_per_sample = 16u16;

    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_len = (audio.samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    // RIFF Header
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // "fmt " chunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    out.extend_from_slice(&1u16.to_le_bytes());  // Audio format 1: PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());

    // "data" chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &sample in &audio.samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }

    out
}

/// Tính năng lượng RMS (Root Mean Square) theo decibel (dB) cho một cửa sổ mẫu
pub fn calculate_window_rms_db(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let mut sum_squares = 0.0f64;
    for &s in samples {
        let norm = s as f64 / 32768.0;
        sum_squares += norm * norm;
    }

    let mean_square = sum_squares / samples.len() as f64;
    let rms = mean_square.sqrt();

    if rms <= 1e-9 {
        -100.0
    } else {
        (20.0 * rms.log10()) as f32
    }
}

/// Cắt bỏ các đoạn im lặng dưới ngưỡng dB
pub fn trim_silence(
    audio: &PcmAudio,
    silence_threshold_db: f32,
    min_silence_duration_ms: u32,
) -> PcmAudio {
    let window_duration_ms = 50u32;
    let samples_per_window = ((audio.sample_rate * window_duration_ms) / 1000) as usize * audio.channels as usize;

    if samples_per_window == 0 || audio.samples.len() < samples_per_window {
        return audio.clone();
    }

    let num_windows = audio.samples.len() / samples_per_window;
    let min_silent_windows = min_silence_duration_ms.div_ceil(window_duration_ms) as usize;

    // Phân loại từng cửa sổ: im lặng hay có âm thanh
    let mut is_silent_window = Vec::with_capacity(num_windows);
    for w in 0..num_windows {
        let start = w * samples_per_window;
        let end = start + samples_per_window;
        let win_samples = &audio.samples[start..end];
        let rms_db = calculate_window_rms_db(win_samples);
        is_silent_window.push(rms_db < silence_threshold_db);
    }

    // Đánh dấu các khối im lặng liên tiếp đạt thời lượng tối thiểu để cắt
    let mut keep_window = vec![true; num_windows];
    let mut i = 0;
    while i < num_windows {
        if is_silent_window[i] {
            let mut j = i;
            while j < num_windows && is_silent_window[j] {
                j += 1;
            }
            let silent_count = j - i;
            if silent_count >= min_silent_windows {
                for item in &mut keep_window[i..j] {
                    *item = false;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }

    // Ghép các cửa sổ giữ lại
    let mut trimmed_samples = Vec::new();
    for (w, &keep) in keep_window.iter().enumerate().take(num_windows) {
        if keep {
            let start = w * samples_per_window;
            let end = start + samples_per_window;
            trimmed_samples.extend_from_slice(&audio.samples[start..end]);
        }
    }

    // Giữ phần mẫu dư cuối cùng nếu có
    let remainder_start = num_windows * samples_per_window;
    if remainder_start < audio.samples.len() {
        let last_kept = keep_window.last().copied().unwrap_or(true);
        if last_kept {
            trimmed_samples.extend_from_slice(&audio.samples[remainder_start..]);
        }
    }

    PcmAudio {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        samples: trimmed_samples,
    }
}

/// Cắt âm thanh theo khoảng thời gian thủ công (start_ms đến end_ms)
pub fn trim_by_time_range(audio: &PcmAudio, start_ms: u32, end_ms: Option<u32>) -> PcmAudio {
    let channels = audio.channels.max(1) as usize;
    let sample_rate = audio.sample_rate.max(8000) as usize;
    let total_frames = audio.samples.len() / channels;

    let start_frame = ((start_ms as usize).saturating_mul(sample_rate) / 1000).min(total_frames);
    let end_frame = match end_ms {
        Some(end) => ((end as usize).saturating_mul(sample_rate) / 1000).clamp(start_frame, total_frames),
        None => total_frames,
    };

    let start_sample = start_frame * channels;
    let end_sample = end_frame * channels;

    let trimmed_samples = if start_sample < end_sample && start_sample < audio.samples.len() {
        audio.samples[start_sample..end_sample.min(audio.samples.len())].to_vec()
    } else {
        Vec::new()
    };

    PcmAudio {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        samples: trimmed_samples,
    }
}

/// Khuếch đại âm lượng (Volume Booster) kèm Soft-clipping Limiter chống vỡ tiếng
pub fn boost_volume_with_limiter(audio: &mut PcmAudio, boost_db: f32, enable_limiter: bool) {
    if boost_db <= 0.0 {
        return;
    }
    let gain = 10.0f32.powf(boost_db / 20.0);

    for sample in &mut audio.samples {
        let val = *sample as f32 * gain;
        if enable_limiter {
            let norm = val / 32768.0;
            let limited = if norm.abs() > 0.8 {
                let sign = norm.signum();
                let excess = norm.abs() - 0.8;
                sign * (0.8 + 0.2 * (excess / 0.2).tanh())
            } else {
                norm
            };
            *sample = (limited.clamp(-1.0, 1.0) * 32767.0) as i16;
        } else {
            *sample = val.clamp(-32768.0, 32767.0) as i16;
        }
    }
}

/// Hợp nhất/Ghép nối nhiều đoạn âm thanh PCM thành một luồng duy nhất
pub fn join_audio(tracks: &[PcmAudio]) -> Result<PcmAudio, CommandError> {
    if tracks.is_empty() {
        return Err(CommandError::ValidationFailed {
            reason: "Danh sách track âm thanh rỗng".to_string(),
        });
    }
    let base = &tracks[0];
    let mut merged_samples = Vec::new();
    for track in tracks {
        if track.channels != base.channels || track.sample_rate != base.sample_rate {
            return Err(CommandError::ValidationFailed {
                reason: format!(
                    "Không thể ghép file khác thông số: {} kênh/{}Hz vs {} kênh/{}Hz",
                    base.channels, base.sample_rate, track.channels, track.sample_rate
                ),
            });
        }
        merged_samples.extend_from_slice(&track.samples);
    }
    Ok(PcmAudio {
        channels: base.channels,
        sample_rate: base.sample_rate,
        samples: merged_samples,
    })
}

impl OfficeTool for AudioSilenceTrimmerTool {
    fn slug(&self) -> &'static str {
        "audio-silence-trimmer"
    }

    fn name(&self) -> &'static str {
        "Cắt Bỏ Khoảng lặng Ghi âm"
    }

    fn description(&self) -> &'static str {
        "Giải mã luồng âm thanh bằng symphonia, tính toán năng lượng RMS theo cửa sổ trượt 50 ms, loại bỏ các khoảng lặng dưới ngưỡng decibel (ví dụ -40 dB) để cô đọng nội dung cuộc họp."
    }

    fn execute(
        &self,
        request: &DispatchTaskRequest,
        cancel_flag: &AtomicBool,
        on_progress: Option<&dyn Fn(TaskProgress)>,
    ) -> Result<DispatchTaskResponse, CommandError> {
        let start_time = Instant::now();

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(CommandError::TaskCancelled);
        }

        let options: AudioTrimOptions = serde_json::from_value(request.options.clone())
            .map_err(|e| CommandError::ValidationFailed {
                reason: format!("Cấu hình tùy chọn AudioTrim không hợp lệ: {e}"),
            })?;

        if request.input_paths.is_empty() {
            return Err(CommandError::ValidationFailed {
                reason: "Danh sách tệp âm thanh đầu vào trống".to_string(),
            });
        }

        let output_dir = request
            .output_directory
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("office_audio_output"));

        fs::create_dir_all(&output_dir).map_err(|e| CommandError::IoError {
            details: format!("Không thể tạo thư mục đầu ra: {e}"),
        })?;

        let mut output_paths = Vec::new();
        let mut failed_items = Vec::new();
        let total_files = request.input_paths.len();

        for (idx, file_path_str) in request.input_paths.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            let path = Path::new(file_path_str);
            if !path.exists() {
                if total_files == 1 {
                    return Err(CommandError::FileNotFound {
                        path: file_path_str.clone(),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: "Tệp không tồn tại".to_string(),
                });
                continue;
            }

            let bytes = match fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    if total_files == 1 {
                        return Err(CommandError::IoError {
                            details: format!("Lỗi đọc tệp: {e}"),
                        });
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("Lỗi đọc: {e}"),
                    });
                    continue;
                }
            };

            let audio = match decode_audio(&bytes) {
                Ok(a) => a,
                Err(err) => {
                    if total_files == 1 {
                        return Err(err);
                    }
                    failed_items.push(FileFailureDetail {
                        input_path: file_path_str.clone(),
                        reason: format!("{:?}", err),
                    });
                    continue;
                }
            };

            if cancel_flag.load(Ordering::Relaxed) {
                return Err(CommandError::TaskCancelled);
            }

            // Bước 1: Cắt theo mốc thời gian thủ công hoặc cắt bỏ khoảng lặng
            let mut processed = if options.manual_trim_enabled
                || options.start_time_ms.is_some()
                || options.end_time_ms.is_some()
            {
                let start = match options.start_time_ms {
                    Some(s) => s,
                    None => 0,
                };
                trim_by_time_range(&audio, start, options.end_time_ms)
            } else {
                trim_silence(
                    &audio,
                    options.silence_threshold_db,
                    options.min_silence_duration_ms,
                )
            };

            // Bước 2: Khuếch đại âm lượng nếu có yêu cầu Volume Booster
            if let Some(boost_db) = options.volume_boost_db {
                if boost_db > 0.0 {
                    boost_volume_with_limiter(&mut processed, boost_db, options.enable_limiter);
                }
            }

            let out_bytes = encode_wav(&processed);
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("trimmed_audio");
            let out_name = format!("{stem}_trimmed.wav");
            let out_path = output_dir.join(out_name);

            if let Err(e) = fs::write(&out_path, out_bytes) {
                if total_files == 1 {
                    return Err(CommandError::IoError {
                        details: format!("Lỗi ghi tệp âm thanh: {e}"),
                    });
                }
                failed_items.push(FileFailureDetail {
                    input_path: file_path_str.clone(),
                    reason: format!("Lỗi ghi: {e}"),
                });
                continue;
            }

            output_paths.push(out_path.to_string_lossy().to_string());

            if let Some(prog) = on_progress {
                prog(TaskProgress {
                    task_id: request.task_id.clone(),
                    current_step: (idx + 1) as u32,
                    total_steps: total_files as u32,
                    percentage: ((idx + 1) as f32 / total_files as f32) * 100.0,
                    message: format!("Đã cắt khoảng lặng file {}/{}", idx + 1, total_files),
                    processed_files: idx + 1,
                    failed_files: failed_items.len(),
                    total_files,
                    is_completed: idx + 1 == total_files,
                });
            }
        }

        let status = if failed_items.is_empty() {
            "SUCCESS".to_string()
        } else if !output_paths.is_empty() {
            "PARTIAL_SUCCESS".to_string()
        } else {
            "FAILED".to_string()
        };

        Ok(DispatchTaskResponse {
            task_id: request.task_id.clone(),
            status,
            output_paths,
            failed_items,
            elapsed_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Tạo file WAV mẫu gồm 1 đoạn có tiếng (sin wave ~ -10 dB) và 1 đoạn im lặng (-90 dB)
    fn create_test_wav_with_silence(sample_rate: u32, tone_ms: u32, silence_ms: u32) -> Vec<u8> {
        let tone_samples = (sample_rate * tone_ms / 1000) as usize;
        let silence_samples = (sample_rate * silence_ms / 1000) as usize;

        let mut samples = Vec::with_capacity(tone_samples * 2 + silence_samples);

        // Đoạn 1: Tiếng nói / âm thanh giả lập (sin wave)
        for i in 0..tone_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            samples.push((val * 16000.0) as i16); // ~ -6 dB
        }

        // Đoạn 2: Khoảng lặng (silence: giá trị gần 0)
        samples.resize(samples.len() + silence_samples, 0);

        // Đoạn 3: Tiếng nói tiếp theo
        for i in 0..tone_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (t * 880.0 * 2.0 * std::f32::consts::PI).sin();
            samples.push((val * 16000.0) as i16);
        }

        let audio = PcmAudio {
            channels: 1,
            sample_rate,
            samples,
        };

        encode_wav(&audio)
    }

    #[test]
    fn test_audio_silence_trimmer_happy_path() {
        let tool = AudioSilenceTrimmerTool;
        let tmp = std::env::temp_dir().join("test_audio_happy");
        let _ = fs::create_dir_all(&tmp);

        // 100ms tone, 500ms silence, 100ms tone
        let wav_bytes = create_test_wav_with_silence(16000, 100, 500);
        let audio = decode_wav(&wav_bytes).expect("decode wav");
        assert_eq!(audio.sample_rate, 16000);
        assert_eq!(audio.channels, 1);

        // Đo RMS cửa sổ 50ms đầu tiên (có tone) phải > -20 dB
        let window_50ms = (16000 * 50) / 1000;
        let rms_tone = calculate_window_rms_db(&audio.samples[0..window_50ms]);
        assert!(rms_tone > -20.0, "Tone window RMS phải > -20 dB");

        // Đo RMS khoảng lặng phải < -60 dB
        let silence_start = (16000 * 200) / 1000;
        let rms_silence = calculate_window_rms_db(&audio.samples[silence_start..silence_start + window_50ms]);
        assert!(rms_silence < -60.0, "Silence window RMS phải < -60 dB");

        let input_file = tmp.join("meeting.wav");
        fs::write(&input_file, wav_bytes).expect("write meeting");

        let req = DispatchTaskRequest {
            task_id: "task-audio-happy".to_string(),
            tool_slug: "audio-silence-trimmer".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "silence_threshold_db": -40.0,
                "min_silence_duration_ms": 300
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("trim should succeed");
        assert_eq!(res.status, "SUCCESS");
        assert_eq!(res.output_paths.len(), 1);
    }

    #[test]
    fn test_audio_silence_trimmer_edge_case() {
        let tool = AudioSilenceTrimmerTool;
        let tmp = std::env::temp_dir().join("test_audio_edge");
        let _ = fs::create_dir_all(&tmp);

        // 200ms tone + 800ms silence + 200ms tone = 1200ms (19200 samples at 16kHz)
        let original_bytes = create_test_wav_with_silence(16000, 200, 800);
        let orig_audio = decode_wav(&original_bytes).expect("decode");

        let input_file = tmp.join("podcast_silence.wav");
        fs::write(&input_file, &original_bytes).expect("write");

        let req = DispatchTaskRequest {
            task_id: "task-audio-trim-edge".to_string(),
            tool_slug: "audio-silence-trimmer".to_string(),
            input_paths: vec![input_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "silence_threshold_db": -40.0,
                "min_silence_duration_ms": 300
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("trim should succeed");
        assert_eq!(res.status, "SUCCESS");

        let trimmed_bytes = fs::read(&res.output_paths[0]).expect("read trimmed");
        let trimmed_audio = decode_wav(&trimmed_bytes).expect("decode trimmed");

        // Đoạn im lặng 800ms đã bị loại bỏ -> số mẫu giảm đi đáng kể
        assert!(trimmed_audio.samples.len() < orig_audio.samples.len());
        // Dự kiến giữ lại khoảng 400ms (khoảng 6400 samples)
        assert!(trimmed_audio.samples.len() <= 8000);
    }

    #[test]
    fn test_audio_silence_trimmer_invalid_magic_bytes() {
        let tool = AudioSilenceTrimmerTool;
        let tmp = std::env::temp_dir().join("test_audio_invalid");
        let _ = fs::create_dir_all(&tmp);

        let bad_file = tmp.join("bad.wav");
        fs::write(&bad_file, b"NOT_A_VALID_RIFF_WAVE_HEADER_DATA").expect("write bad");

        let req = DispatchTaskRequest {
            task_id: "task-bad-audio".to_string(),
            tool_slug: "audio-silence-trimmer".to_string(),
            input_paths: vec![bad_file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "silence_threshold_db": -40.0 }),
        };

        let cancel = AtomicBool::new(false);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_audio_silence_trimmer_cooperative_cancellation() {
        let tool = AudioSilenceTrimmerTool;
        let tmp = std::env::temp_dir().join("test_audio_cancel");
        let _ = fs::create_dir_all(&tmp);

        let file = tmp.join("cancel.wav");
        fs::write(&file, create_test_wav_with_silence(8000, 50, 50)).expect("write");

        let req = DispatchTaskRequest {
            task_id: "task-cancel-audio".to_string(),
            tool_slug: "audio-silence-trimmer".to_string(),
            input_paths: vec![file.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({ "silence_threshold_db": -40.0 }),
        };

        let cancel = AtomicBool::new(true);
        let err = tool.execute(&req, &cancel, None).unwrap_err();
        assert_eq!(err, CommandError::TaskCancelled);
    }

    #[test]
    fn test_audio_silence_trimmer_security_guardrails() {
        let tool = AudioSilenceTrimmerTool;
        assert_eq!(tool.slug(), "audio-silence-trimmer");
        assert_eq!(tool.name(), "Cắt Bỏ Khoảng lặng Ghi âm");

        // Kiểm tra xử lý file rỗng hoặc ngắn hơn 44 bytes
        let short_bytes = b"RIFF";
        let err = decode_wav(short_bytes).unwrap_err();
        match err {
            CommandError::InvalidFileSignature { .. } => {}
            other => panic!("Kỳ vọng InvalidFileSignature cho short file nhưng nhận được: {:?}", other),
        }
    }

    #[test]
    fn test_backward_compatible_preset_deserialization() {
        // Kiểm tra nạp cấu hình cũ chỉ có 2 trường (không có manual trim & volume boost)
        let old_json = json!({
            "silence_threshold_db": -35.0,
            "min_silence_duration_ms": 400
        });

        let opts: AudioTrimOptions = serde_json::from_value(old_json).expect("deserialize old json");
        assert_eq!(opts.silence_threshold_db, -35.0);
        assert_eq!(opts.min_silence_duration_ms, 400);
        assert!(!opts.manual_trim_enabled);
        assert_eq!(opts.start_time_ms, None);
        assert_eq!(opts.end_time_ms, None);
        assert_eq!(opts.volume_boost_db, None);
        assert!(!opts.enable_limiter);
    }

    #[test]
    fn test_audio_manual_trim_and_volume_boost() {
        let tool = AudioSilenceTrimmerTool;
        let tmp = std::env::temp_dir().join("test_audio_manual_trim");
        let _ = fs::create_dir_all(&tmp);

        // Tạo tệp wav 1000ms (8000Hz, mono -> 8000 samples)
        let sample_rate = 8000u32;
        let mut samples = Vec::new();
        for i in 0..8000 {
            let val = (1000.0 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate as f32).sin()) as i16;
            samples.push(val);
        }
        let initial_audio = PcmAudio {
            channels: 1,
            sample_rate,
            samples,
        };
        let wav_bytes = encode_wav(&initial_audio);
        let file_path = tmp.join("tone.wav");
        fs::write(&file_path, wav_bytes).expect("write tone wav");

        // 1. Kiểm thử cắt thủ công: từ 100ms đến 300ms (200ms = 1600 samples)
        let trimmed_audio = trim_by_time_range(&initial_audio, 100, Some(300));
        assert_eq!(trimmed_audio.samples.len(), 1600);

        // 2. Kiểm thử Volume Booster với Limiter
        let mut boosted_audio = trimmed_audio.clone();
        boost_volume_with_limiter(&mut boosted_audio, 12.0, true);
        assert_eq!(boosted_audio.samples.len(), 1600);
        // Biên độ phải tăng lên nhưng không bị vượt quá giới hạn i16
        let max_orig = trimmed_audio.samples.iter().map(|s| s.abs()).max().unwrap_or(0);
        let max_boosted = boosted_audio.samples.iter().map(|s| s.abs()).max().unwrap_or(0);
        assert!(max_boosted > max_orig);
        assert!(max_boosted > 0);

        // 3. Kiểm thử qua execute() với options đầy đủ
        let req = DispatchTaskRequest {
            task_id: "task-manual-trim-test".to_string(),
            tool_slug: "audio-silence-trimmer".to_string(),
            input_paths: vec![file_path.to_string_lossy().to_string()],
            output_directory: Some(tmp.to_string_lossy().to_string()),
            options: json!({
                "manual_trim_enabled": true,
                "start_time_ms": 100,
                "end_time_ms": 500,
                "volume_boost_db": 6.0,
                "enable_limiter": true
            }),
        };

        let cancel = AtomicBool::new(false);
        let res = tool.execute(&req, &cancel, None).expect("execute manual trim");
        assert_eq!(res.status, "SUCCESS");

        let out_bytes = fs::read(&res.output_paths[0]).expect("read out wav");
        let out_audio = decode_wav(&out_bytes).expect("decode out wav");
        // 400ms = 3200 samples
        assert_eq!(out_audio.samples.len(), 3200);

        // 4. Kiểm thử ghép file join_audio
        let joined = join_audio(&[out_audio.clone(), out_audio.clone()]).expect("join audio");
        assert_eq!(joined.samples.len(), 6400);
    }
}
