#!/usr/bin/env python3
import sys
import time
import os
import zipfile
import hashlib
import numpy as np
from PIL import Image

sys.path.append('/home/quan/workspace/tauri-v2/office-utility-vn/stage_4c0c_evidence/scripts')
from gui_driver import GuiDriver

def compute_sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def main():
    print("=== STARTING RECONCILED REAL DESKTOP GUI TC-GUI-04 E2E EXECUTION ===")
    src_fixture = "/home/quan/workspace/tauri-v2/office-utility-vn/fixtures/test_uat_sample.xlsx"
    fixture_path = "/tmp/office-utility-uat/tc04_sample.xlsx"
    import shutil
    os.makedirs("/tmp/office-utility-uat", exist_ok=True)
    shutil.copyfile(src_fixture, fixture_path)
    orig_hash = compute_sha256(fixture_path)
    print(f"[Fixture] Freshly copied from {src_fixture}")
    print(f"[Fixture] Original hash: {orig_hash}")

    driver = GuiDriver()
    
    # 1. Wait for window to appear
    print("[Step 1] Waiting for Tauri desktop window...")
    w = None
    for i in range(60):
        w = driver.find_window()
        if w:
            break
        time.sleep(1)
    
    if not w:
        print("ERROR: Window did not appear within 60s")
        sys.exit(1)

    print(f"[Window] Found window: {hex(w.id)}")

    # 2. Wait for Leptos UI to mount and render GridCraft Dev button
    print("[Step 2] Waiting for Leptos UI to mount and render 'GridCraft Dev' button...")
    dev_btn_pos = (585, 18)
    mounted = False
    for i in range(30):
        driver.screenshot("/tmp/office-utility-uat/tc04_e2e_00_launcher.png")
        try:
            im = Image.open("/tmp/office-utility-uat/tc04_e2e_00_launcher.png").convert("RGB")
            arr = np.array(im)
            green_mask = (arr[:40, :, 0] < 80) & (arr[:40, :, 1] > 180) & (arr[:40, :, 2] > 120)
            cnt = np.sum(green_mask)
            if cnt > 10:
                print(f"✓ Leptos UI mounted! Found 'GridCraft Dev' button with {cnt} green pixels.")
                y_idx, x_idx = np.where(green_mask)
                center_x = int(np.mean(x_idx))
                center_y = int(np.mean(y_idx))
                dev_btn_pos = (center_x, center_y)
                print(f"✓ 'GridCraft Dev' button center resolved at {dev_btn_pos}")
                mounted = True
                break
        except Exception as e:
            print(f"Wait error: {e}")
        time.sleep(1)

    if not mounted:
        print("WARNING: Could not find green button by color, using fallback (585, 18)")

    # 3. Click "🧪 GridCraft Dev" button
    print(f"[Step 3] Clicking 'GridCraft Dev' button at {dev_btn_pos}...")
    driver.click(dev_btn_pos[0], dev_btn_pos[1])

    # 4. Wait for GridCraft spreadsheet grid (WASM egui) to render and open fixture
    print("[Step 4] Waiting for GridCraft WASM egui spreadsheet grid to render and open fixture...")
    grid_rendered = False
    for i in range(30):
        driver.screenshot("/tmp/office-utility-uat/tc04_e2e_01_loaded.png")
        try:
            im = Image.open("/tmp/office-utility-uat/tc04_e2e_01_loaded.png").convert("RGB")
            arr = np.array(im)
            top_bar_mean = arr[45:70, 100:200].mean()
            # If top bar is not dark yet, click Dev button again
            if top_bar_mean > 100 and i in (2, 5):
                print(f"Top bar still light (mean={top_bar_mean:.1f}), re-clicking Dev button...")
                driver.click(dev_btn_pos[0], dev_btn_pos[1])
            elif top_bar_mean < 60:
                # Check for spreadsheet grid lines or text
                grid_mask = (arr[180:220, 40:300, 0] < 60) & (arr[180:220, 40:300, 1] < 60)
                if np.sum(grid_mask) > 50:
                    print(f"✓ GridCraft spreadsheet fully rendered and fixture loaded! (text pixels: {np.sum(grid_mask)})")
                    grid_rendered = True
                    break
        except Exception as e:
            print(f"Wait error: {e}")
        time.sleep(1)

    if not grid_rendered:
        print("ERROR: GridCraft spreadsheet grid failed to render within 30s")
        sys.exit(1)

    # Allow 4.0 seconds for postMessage gridcraft:open, parsing and WASM canvas quiescence
    print("[Step 4b] Waiting 4.0s for GridCraft WASM engine to become completely quiescent...")
    time.sleep(4.0)
    driver.screenshot("/tmp/office-utility-uat/tc04_e2e_01_loaded.png")

    # 5. Double-click Cell A2 at (60, 205) and type edit
    # Reconciled coordinates in 800x600 window:
    # Row 1 (A1='OfficeUtilityVN_UAT_Sample', B1=12345): y = 176..198
    # Row 2 (A2): y = 199..214 -> center y = 205, x = 60 (Column A)
    # Double-click triggers GridCraft's handle_cell_double_click (app.begin_edit)
    # F2 also activates in-cell editing mode in GridCraft egui
    # Coordinate (150, 205) clicks Cell C2 to commit the cell draft
    print("[Step 5] Editing Cell A2 at (60, 205) with exact 'TC04_E2E_SAVED'...")
    dirty_confirmed = False
    for attempt in range(1, 4):
        print(f"--- Edit Attempt {attempt}/3 ---")
        driver.double_click(60, 205)
        time.sleep(0.3)
        driver.send_key("F2")
        time.sleep(0.3)
        driver.type_text("TC04_E2E_SAVED")
        time.sleep(0.5)
        driver.send_key("Return")
        time.sleep(0.5)
        driver.click(150, 205)
        time.sleep(1.0)

        driver.screenshot("/tmp/office-utility-uat/tc04_e2e_02_edited.png")
        try:
            im = Image.open("/tmp/office-utility-uat/tc04_e2e_02_edited.png").convert("RGB")
            arr = np.array(im)
            crop = arr[38:74, 150:400]
            yellow_mask = (crop[:, :, 0] > 180) & (crop[:, :, 1] > 140) & (crop[:, :, 2] < 120)
            yellow_cnt = np.sum(yellow_mask)
            print(f"Edit attempt {attempt}: yellow pixels in dirty badge = {yellow_cnt}")
            if yellow_cnt > 10:
                print(f"✓ SUCCESS: Dirty badge '● Chưa lưu' confirmed! ({yellow_cnt} yellow pixels)")
                dirty_confirmed = True
                break
        except Exception as e:
            print(f"Check error: {e}")
        time.sleep(1.0)

    if not dirty_confirmed:
        print("ERROR: Document was not marked dirty after 3 edit attempts!")
        sys.exit(1)

    # 7. Trigger window close via Close button (775, 18)
    print("[Step 11] Triggering Window Close request at (775, 18)...")
    driver.click(775, 18)
    time.sleep(1.5)

    # 8. Wait for Unsaved Changes confirmation modal
    print("[Step 12] Waiting for Unsaved Changes confirmation modal...")
    modal_detected = False
    for i in range(15):
        driver.screenshot("/tmp/office-utility-uat/tc04_e2e_03_modal.png")
        try:
            im = Image.open("/tmp/office-utility-uat/tc04_e2e_03_modal.png").convert("RGB")
            arr = np.array(im)
            # Check for green Save button around (538, 350) (y: 335-365, x: 480-600)
            save_btn = arr[335:365, 480:600]
            green_mask = (save_btn[:, :, 0] < 80) & (save_btn[:, :, 1] > 140) & (save_btn[:, :, 2] < 120)
            green_cnt = np.sum(green_mask)
            if green_cnt > 30:
                print(f"✓ Unsaved Changes modal detected! (Save button green pixels: {green_cnt})")
                modal_detected = True
                break
        except Exception as e:
            print(f"Wait error: {e}")
        time.sleep(0.5)

    if not modal_detected:
        print("ERROR: Unsaved Changes modal was NOT displayed! Window close was not guarded.")
        sys.exit(1)

    # 9. Click "Lưu & Thoát (Save)" button at (538, 350)
    print("[Step 13] Clicking 'Lưu & Thoát (Save)' button at (538, 350)...")
    driver.click(538, 350)

    # 10. Wait for asynchronous persistence and graceful window termination
    print("[Step 14] Waiting for asynchronous persistence and graceful window termination...")
    closed_cleanly = False
    for i in range(25):
        w = driver.find_window()
        if w is None:
            print("✓ SUCCESS: Window terminated cleanly after Save & Exit authorization!")
            closed_cleanly = True
            break
        time.sleep(0.5)

    if not closed_cleanly:
        print("ERROR: Window did not terminate within timeout after Save click!")
        sys.exit(1)

    # 11. Verify file durability and independent XLSX content
    print("[Step 15] Verifying file durability and independent XLSX contents via openpyxl...")
    new_hash = compute_sha256(fixture_path)
    print(f"[Durability Check] New hash: {new_hash}")
    if new_hash == orig_hash:
        print("ERROR: Fixture hash unchanged! Save did not write to disk.")
        sys.exit(1)
    else:
        print(f"✓ SUCCESS: Fixture modified on disk! Hash changed: {orig_hash} -> {new_hash}")

    import openpyxl
    wb = openpyxl.load_workbook(fixture_path, data_only=False)
    sheet_names = wb.sheetnames
    print(f"[XLSX Check] Sheet names: {sheet_names}")
    if 'UAT_Sheet' not in sheet_names:
        print(f"ERROR: Expected sheet 'UAT_Sheet' not found! Got: {sheet_names}")
        sys.exit(1)
    else:
        print("✓ SUCCESS: Sheet 'UAT_Sheet' preserved intact!")

    sheet = wb['UAT_Sheet']
    val_a1 = sheet['A1'].value
    val_b1 = sheet['B1'].value
    val_a2 = sheet['A2'].value
    print(f"[XLSX Check] Cell A1 value: {val_a1!r} (expected: 'OfficeUtilityVN_UAT_Sample' - unmodified intact)")
    print(f"[XLSX Check] Cell B1 value: {val_b1!r} (expected: 12345 - unmodified intact)")
    print(f"[XLSX Check] Cell A2 value: {val_a2!r} (expected: 'TC04_E2E_SAVED' - exact uppercase edited value)")

    if val_a1 != "OfficeUtilityVN_UAT_Sample":
        print(f"ERROR: Cell A1 value corrupted! Got {val_a1!r}, expected 'OfficeUtilityVN_UAT_Sample'")
        sys.exit(1)
    else:
        print("✓ SUCCESS: Cell A1 preserved unmodified original content: 'OfficeUtilityVN_UAT_Sample'!")

    if val_b1 != 12345:
        print(f"ERROR: Cell B1 value corrupted! Got {val_b1!r}, expected 12345")
        sys.exit(1)
    else:
        print("✓ SUCCESS: Cell B1 preserved unmodified original content: 12345!")

    if val_a2 != "TC04_E2E_SAVED":
        print(f"ERROR: Cell A2 value mismatch! Got {val_a2!r}, expected 'TC04_E2E_SAVED'")
        sys.exit(1)
    else:
        print("✓ SUCCESS: Cell A2 matches exact uppercase 'TC04_E2E_SAVED'!")

    # Save artifact copy
    import shutil
    artifact_saved_path = "/home/quan/workspace/tauri-v2/office-utility-vn/stage_4c0c_evidence/artifacts/tc04_postfix_saved.xlsx"
    shutil.copyfile(fixture_path, artifact_saved_path)
    print(f"✓ Saved verified artifact copy to {artifact_saved_path}")

    # Step 17: Run independent Calamine verification via cargo test
    print("[Step 17] Running independent Rust Calamine engine verification...")
    import subprocess
    cal_res = subprocess.run([
        "cargo", "test", "-p", "office-utility-vn", "--lib",
        "test_calamine_verify_postfix_saved_workbook", "--", "--nocapture"
    ], cwd="/home/quan/workspace/tauri-v2/office-utility-vn", capture_output=True, text=True)
    print(cal_res.stdout)
    if cal_res.returncode != 0:
        print("ERROR: Calamine verification failed!")
        print(cal_res.stderr)
        sys.exit(1)
    else:
        print("✓ SUCCESS: Calamine independent verification passed 100%!")

    print("=== RECONCILED REAL DESKTOP GUI TC-GUI-04 E2E EXECUTION COMPLETED 100% PASS ===")

if __name__ == "__main__":
    main()
