use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = playVietnameseNumberAudio)]
    fn play_vn_number_audio_js(tokens: js_sys::Array, on_ended: &js_sys::Function);

    #[wasm_bindgen(js_name = stopVietnameseNumberAudio)]
    fn stop_vn_number_audio_js();
}

/// Phát danh sách mã âm thanh nối tiếp qua Web Audio API
pub fn play_audio_tokens(tokens: Vec<&'static str>, on_ended: Callback<()>) {
    #[cfg(target_arch = "wasm32")]
    {
        let arr = js_sys::Array::new();
        for t in tokens {
            arr.push(&JsValue::from_str(t));
        }
        let cb = Closure::<dyn Fn()>::new(move || {
            on_ended.run(());
        });
        play_vn_number_audio_js(arr, cb.as_ref().unchecked_ref());
        cb.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = tokens;
        on_ended.run(());
    }
}

/// Dừng phát âm thanh ngay lập tức
pub fn stop_audio() {
    #[cfg(target_arch = "wasm32")]
    {
        stop_vn_number_audio_js();
    }
}

/// Chuyển đổi chuỗi văn bản kết quả tiếng Việt thành danh sách mã token âm thanh tương ứng
pub fn words_to_audio_tokens(words_str: &str, is_south: bool) -> Vec<&'static str> {
    let lower = words_str.to_lowercase();
    let clean = lower.replace(['.', ',', ';', ':', '!', '?'], " ");
    let raw_tokens: Vec<&str> = clean.split_whitespace().collect();

    let mut tokens = Vec::new();
    let mut i = 0;
    while i < raw_tokens.len() {
        let w = raw_tokens[i];
        match w {
            "không" => tokens.push("khong"),
            "một" => {
                if i > 0 && raw_tokens[i - 1] == "mươi" {
                    if is_south {
                        tokens.push("mot_nam");
                    } else {
                        tokens.push("mot_sau");
                    }
                } else {
                    tokens.push("mot");
                }
            }
            "mốt" => {
                if is_south {
                    tokens.push("mot_nam");
                } else {
                    tokens.push("mot_sau");
                }
            }
            "hai" => tokens.push("hai"),
            "ba" => tokens.push("ba"),
            "bốn" => {
                if i > 0 && raw_tokens[i - 1] == "mươi" && is_south {
                    tokens.push("tu_nam");
                } else {
                    tokens.push("bon");
                }
            }
            "tư" => tokens.push("tu"),
            "năm" => tokens.push("nam"),
            "lăm" => tokens.push("lam"),
            "sáu" => tokens.push("sau"),
            "bảy" => tokens.push("bay"),
            "tám" => tokens.push("tam"),
            "chín" => tokens.push("chin"),
            "mười" => tokens.push("muoi"),
            "mươi" => tokens.push("muoi_chuc"),
            "chục" => tokens.push("chuc"),
            "trăm" => tokens.push("tram"),
            "nghìn" => tokens.push("nghin"),
            "ngàn" => tokens.push("ngan"),
            "triệu" => tokens.push("trieu"),
            "tỷ" => tokens.push("ty"),
            "lẻ" => tokens.push("le"),
            "linh" => tokens.push("linh"),
            "âm" => tokens.push("am"),
            "và" => tokens.push("va"),
            "đồng" => tokens.push("dong"),
            "chẵn" => tokens.push("chan"),
            "xu" => tokens.push("xu"),
            "phẩy" => tokens.push("phay"),
            "chấm" => tokens.push("cham"),
            "đô" => {
                if i + 2 < raw_tokens.len() && raw_tokens[i + 1] == "la" && raw_tokens[i + 2] == "mỹ" {
                    tokens.push("do_la_my");
                    i += 2;
                } else {
                    tokens.push("do_la_my");
                }
            }
            "euro" => tokens.push("euro"),
            "cents" | "cent" | "xen" => tokens.push("cents"),
            _ => {}
        }
        i += 1;
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_non_wasm_fallback() {
        let invoked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked_clone = invoked.clone();

        let cb = Callback::new(move |_| {
            invoked_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        play_audio_tokens(vec!["mot", "tram"], cb);
        stop_audio();

        #[cfg(not(target_arch = "wasm32"))]
        assert!(invoked.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_words_to_audio_tokens_shared_ui() {
        let text = "Một triệu hai trăm năm mươi nghìn đồng chẵn";
        let tokens = words_to_audio_tokens(text, false);
        assert_eq!(
            tokens,
            vec!["mot", "trieu", "hai", "tram", "nam", "muoi_chuc", "nghin", "dong", "chan"]
        );

        let south_text = "Một triệu hai trăm năm mươi ngàn đồng chẵn";
        let south_tokens = words_to_audio_tokens(south_text, true);
        assert_eq!(
            south_tokens,
            vec!["mot", "trieu", "hai", "tram", "nam", "muoi_chuc", "ngan", "dong", "chan"]
        );
    }
}
