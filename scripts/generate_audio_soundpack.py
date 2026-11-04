#!/usr/bin/env python3
"""
Tạo bộ mẫu âm thanh phòng thu 35 từ tiếng Việt phục vụ đọc số tiền chuẩn
100% On-Device cho công cụ vietnamese-number-to-words.
"""

import asyncio
from pathlib import Path
import edge_tts

OUTPUT_DIR = Path("public/audio/vn_numbers")

# Danh sách từ vựng đọc số và voice tương ứng (HoaiMy: giọng Bắc chuẩn, NamMinh: giọng Nam chuẩn)
TOKENS = [
    ("khong", "không", "vi-VN-HoaiMyNeural"),
    ("mot", "một", "vi-VN-HoaiMyNeural"),
    ("hai", "hai", "vi-VN-HoaiMyNeural"),
    ("ba", "ba", "vi-VN-HoaiMyNeural"),
    ("bon", "bốn", "vi-VN-HoaiMyNeural"),
    ("nam", "năm", "vi-VN-HoaiMyNeural"),
    ("sau", "sáu", "vi-VN-HoaiMyNeural"),
    ("bay", "bảy", "vi-VN-HoaiMyNeural"),
    ("tam", "tám", "vi-VN-HoaiMyNeural"),
    ("chin", "chín", "vi-VN-HoaiMyNeural"),
    ("mot_sau", "mốt", "vi-VN-HoaiMyNeural"),
    ("tu", "tư", "vi-VN-HoaiMyNeural"),
    ("lam", "lăm", "vi-VN-HoaiMyNeural"),
    ("muoi", "mười", "vi-VN-HoaiMyNeural"),
    ("muoi_chuc", "mươi", "vi-VN-HoaiMyNeural"),
    ("tram", "trăm", "vi-VN-HoaiMyNeural"),
    ("nghin", "nghìn", "vi-VN-HoaiMyNeural"),
    ("ngan", "ngàn", "vi-VN-NamMinhNeural"),
    ("trieu", "triệu", "vi-VN-HoaiMyNeural"),
    ("ty", "tỷ", "vi-VN-HoaiMyNeural"),
    ("le", "lẻ", "vi-VN-HoaiMyNeural"),
    ("linh", "linh", "vi-VN-NamMinhNeural"),
    ("am", "âm", "vi-VN-HoaiMyNeural"),
    ("va", "và", "vi-VN-HoaiMyNeural"),
    ("dong", "đồng", "vi-VN-HoaiMyNeural"),
    ("chan", "chẵn", "vi-VN-HoaiMyNeural"),
    ("xu", "xu", "vi-VN-HoaiMyNeural"),
    ("do_la_my", "đô la Mỹ", "vi-VN-HoaiMyNeural"),
    ("euro", "euro", "vi-VN-HoaiMyNeural"),
    ("cents", "xen", "vi-VN-HoaiMyNeural"),
    ("mot_nam", "mốt", "vi-VN-NamMinhNeural"),
    ("tu_nam", "bốn", "vi-VN-NamMinhNeural"),
    ("chuc", "chục", "vi-VN-HoaiMyNeural"),
    ("phay", "phẩy", "vi-VN-HoaiMyNeural"),
    ("cham", "chấm", "vi-VN-HoaiMyNeural"),
]

async def generate_token(key: str, text: str, voice: str):
    out_file = OUTPUT_DIR / f"{key}.mp3"
    print(f"🎙️ Đang tạo mẫu âm '{key}' ({text}) qua voice {voice}...")
    communicate = edge_tts.Communicate(text, voice)
    await communicate.save(str(out_file))

async def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for k, t, v in TOKENS:
        out_file = OUTPUT_DIR / f"{k}.mp3"
        if out_file.exists() and out_file.stat().st_size > 1000:
            print(f"✓ Đã tồn tại: '{k}.mp3'")
            continue
        try:
            await generate_token(k, t, v)
            await asyncio.sleep(0.3)
        except Exception as e:
            print(f"⚠️ Thử lại '{k}' do lỗi: {e}")
            await asyncio.sleep(1.0)
            await generate_token(k, t, v)
    print(f"✅ Hoàn tất tạo {len(TOKENS)} mẫu âm thanh trong '{OUTPUT_DIR}'!")

if __name__ == "__main__":
    asyncio.run(main())
