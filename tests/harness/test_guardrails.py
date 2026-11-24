"""Self-tests of the guardrail checker: synthetic Rust snippets are appropriate HERE.
These are *not* proof of business Engine correctness and cannot replace real fixtures.
"""
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from verify_guardrails import load_policy, scan_repo  # noqa: E402

POLICY = load_policy(ROOT / "policy/guardrails.json")


class GuardrailMutationTests(unittest.TestCase):
    def scan(self, filename, code):
        with tempfile.TemporaryDirectory() as folder:
            target = Path(folder) / filename
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(code, encoding="utf-8")
            return scan_repo(Path(folder), POLICY, require_layout=False)

    def assert_rule(self, filename, code, expected):
        findings = self.scan(filename, code)
        self.assertIn(expected, {f.rule_id for f in findings}, findings)

    def test_pdf_lossy_detected(self):
        self.assert_rule("crates/office-engine/src/doc_suite/pdf_crop_margin.rs",
                         'let raw_text = String::from_utf8_lossy(bytes);', "PDF001")

    def test_pdf_raw_delimiter_detected(self):
        self.assert_rule("crates/office-engine/src/doc_suite/pdf_crop_margin.rs",
                         'let e = raw.find("endobj");', "PDF002")

    def test_fake_pki_detected(self):
        self.assert_rule("crates/office-engine/src/doc_suite/pdf_pki_sign.rs",
                         'let payload = format!("PKCS7-SIG:CN={}", cn);', "PDF003")

    def test_fake_ocr_detected(self):
        self.assert_rule("crates/office-engine/src/doc_suite/vietnamese_ocr.rs",
                         'return Ok("HÓA ĐƠN GIÁ TRỊ GIA TĂNG".to_string());', "FAKE001")

    def test_fake_video_detected(self):
        self.assert_rule("crates/office-engine/src/media_studio/video_to_gif_webp.rs",
                         'let bar_x = (progress_ratio * base_w as f32) as i32;', "FAKE002")

    def test_fake_pdf_bitmap_detected(self):
        self.assert_rule("crates/office-engine/src/doc_suite/pdf_to_long_image.rs",
                         'if stream_content.contains("Tj") {}', "FAKE003")

    def test_vni_lossy_detected(self):
        self.assert_rule("crates/office-engine/src/data_lab/vietnamese_font_converter.rs",
                         'let x = String::from_utf8_lossy(bytes);', "VNI001")

    def test_zlib_unbounded_detected(self):
        self.assert_rule("crates/office-engine/src/media_studio/image_batch.rs",
                         'decoder.read_to_end(&mut decompressed)?;', "PNG001")

    def test_placeholder_fallback_detected(self):
        self.assert_rule("crates/office-engine/src/media_studio/image_batch.rs",
                         'Ok(RawImage { pixels: vec![255; (w*h*3) as usize] })', "PNG002")

    def test_rename_ignored_detected(self):
        self.assert_rule("crates/office-engine/src/data_lab/base64_file_codec.rs",
                         'let _ = fs::rename(&tmp, &dst);', "FS001")

    def test_session_unwrap_detected(self):
        self.assert_rule("src-tauri/src/safe_persistence.rs",
                         'let s = sessions.get(&doc_id).unwrap();', "SESSION001")

    def test_plain_password_detected(self):
        self.assert_rule("crates/office-engine/src/data_lab/excel_sheet_unlocker.rs",
                         'pub password: Option<String>,', "SECRET001")

    def test_read_error_swallowed_detected(self):
        self.assert_rule("src-tauri/src/commands/mod.rs",
                         'let n = file.read(&mut header).unwrap_or(0);', "IO001")

    def test_ipc_wildcard_fake_success_detected(self):
        code = '''async fn dispatch_tool_task() -> Result<(),()> {
            match x { "good" => { Ok(()) }, _ => {
               let r = Foo { status: "SUCCESS", output_paths: x };
               Ok(())
            }}
        }'''
        self.assert_rule("src-tauri/src/commands/mod.rs", code, "IPC001")

    def test_ipc_explicit_error_is_safe(self):
        code = '''async fn dispatch_tool_task() -> Result<(),()> {
            match x { "good" => { Ok(()) }, _ => {
              Err(CommandError::ToolUnavailable)
            }}
        }'''
        self.assertNotIn("IPC001", {f.rule_id for f in self.scan("src-tauri/src/commands/mod.rs", code)})

    def test_async_blocking_detected(self):
        code = '''pub async fn inspect_file_metadata(p: String) -> () {
             let meta = std::fs::metadata(p);
        }'''
        self.assert_rule("src-tauri/src/commands/mod.rs", code, "IO002")

    def test_async_spawn_blocking_not_flagged(self):
        code = '''pub async fn inspect_file_metadata(p: String) -> () {
             tauri::async_runtime::spawn_blocking(move || { std::fs::metadata(p) }).await;
        }'''
        self.assertNotIn("IO002", {f.rule_id for f in self.scan("src-tauri/src/commands/mod.rs", code)})

    def test_ime_unsafe_detected(self):
        code = '''view! { <input prop:value=foo on:input=move |ev| { update(ev) } /> }'''
        self.assert_rule("src/tools/demo.rs", code, "IME001")

    def test_ime_safe_not_flagged(self):
        code = '''view! { <input prop:value=foo
            on:compositionstart=move |_| composing.set(true)
            on:compositionend=move |_| composing.set(false)
            on:input=move |ev| { update(ev) } /> }'''
        self.assertNotIn("IME001", {f.rule_id for f in self.scan("src/tools/demo.rs", code)})

    def test_full_line_comments_ignored(self):
        code = '''// String::from_utf8_lossy(bytes)
            let doc = lopdf::Document::load_mem(bytes)?;'''
        self.assertEqual([], self.scan("crates/office-engine/src/doc_suite/demo.rs", code))

    def test_unknown_rule_kind_rejected(self):
        modified = json.loads(json.dumps(POLICY))
        modified["rules"][0]["kind"] = "pass_everything"
        with tempfile.TemporaryDirectory() as folder:
            f = Path(folder) / "bad.json"
            f.write_text(json.dumps(modified))
            with self.assertRaises(ValueError):
                load_policy(f)


if __name__ == "__main__":
    unittest.main()
