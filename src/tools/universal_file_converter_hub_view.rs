use leptos::prelude::*;

/// Thẻ công cụ Trạm Chuyển đổi Tệp Vạn năng (Universal File Converter Hub).
///
/// Hỗ trợ chuyển đổi linh hoạt giữa các định dạng:
/// PDF, Word (.docx), Excel (.xlsx), CSV, JSON, SQL INSERT, Markdown (.md), TXT và Hình ảnh.
#[component]
pub fn UniversalFileConverterHubView(
    #[prop(default = signal("xlsx".to_string()).0)] target_format: ReadSignal<String>,
    #[prop(default = signal("xlsx".to_string()).1)] set_target_format: WriteSignal<String>,
    #[prop(default = signal(true).0)] preserve_formatting: ReadSignal<bool>,
    #[prop(default = signal(true).1)] set_preserve_formatting: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="tool-config-panel">
            <div class="options-row" style="display: flex; flex-direction: column; gap: 12px;">
                <div class="form-group">
                    <label class="form-label">"🎯 Định dạng đích mong muốn:"</label>
                    <select
                        class="form-select"
                        on:change=move |ev| set_target_format.set(event_target_value(&ev))
                        prop:value=move || target_format.get()
                    >
                        <option value="xlsx">"Bảng tính Excel (.xlsx) — Bóc tách bảng biểu số liệu"</option>
                        <option value="csv">"Bảng dữ liệu CSV (.csv) — Định dạng phân tách phẩy UTF-8"</option>
                        <option value="json">"Cấu trúc JSON (.json) — Mảng đối tượng dữ liệu"</option>
                        <option value="sql">"Kịch bản SQL INSERT (.sql) — Nạp cơ sở dữ liệu"</option>
                        <option value="md">"Tài liệu Markdown (.md) — Cấu trúc tiêu đề & bảng kẻ"</option>
                        <option value="txt">"Văn bản thuần (.txt) — Bóc tách nội dung thô hoặc OCR"</option>
                        <option value="pdf">"Tài liệu PDF (.pdf) — Khổ A4 chuẩn công vụ Việt Nam"</option>
                        <option value="docx">"Tài liệu Word (.docx) — Microsoft Word OpenXML"</option>
                    </select>
                </div>

                <div class="form-group">
                    <label class="radio-item">
                        <input
                            type="checkbox"
                            checked=move || preserve_formatting.get()
                            on:change=move |ev| set_preserve_formatting.set(event_target_checked(&ev))
                        />
                        <span>"Bảo toàn tối đa định dạng ô bảng, kiểu số liệu và phông chữ Unicode tiếng Việt"</span>
                    </label>
                </div>
            </div>

            <div class="matrix-empty-guide" style="margin-top: 14px; padding: 12px; background: rgba(15, 23, 42, 0.6); border: 1px solid #334155; border-radius: 8px;">
                <div style="font-size: 12px; font-weight: 600; color: #94A3B8; margin-bottom: 8px;">
                    "⚡ Ma trận Chuyển đổi Tệp Vạn năng (100% On-Device):"
                </div>
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 8px; font-size: 11px;">
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #EF4444; font-weight: 600;">"📄 PDF "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"Excel (.xlsx), CSV, Markdown, OCR Text"</span>
                    </div>
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #3B82F6; font-weight: 600;">"📘 Word (.docx) "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"Markdown (.md), Text thuần, PDF"</span>
                    </div>
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #10B981; font-weight: 600;">"📗 Excel (.xlsx) "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"Bảng CSV, JSON, SQL INSERT, Markdown"</span>
                    </div>
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #06B6D4; font-weight: 600;">"📊 CSV "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"JSON, SQL INSERT, Excel .xlsx"</span>
                    </div>
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #A855F7; font-weight: 600;">"📝 Markdown / TXT "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"PDF Chuẩn Công vụ VN (A4), Unicode NFC"</span>
                    </div>
                    <div style="padding: 6px 8px; background: #1E293B; border-radius: 4px; border: 1px solid #334155;">
                        <span style="color: #F59E0B; font-weight: 600;">"🖼️ Hình ảnh "</span>
                        <span style="color: #64748B;">"➔ "</span>
                        <span style="color: #CBD5E1;">"WebP, PNG không vỡ nét, JPEG, BMP"</span>
                    </div>
                </div>
            </div>

            <div style="font-size: 11px; color: var(--text-muted); margin-top: 10px;">
                "🔒 100% On-Device: Không gửi dữ liệu qua Internet. Tự động nhận diện chữ ký file (Magic bytes)."
            </div>
        </div>
    }
}
