# Office Utility VN

Ứng dụng desktop đa nền tảng được xây dựng với [Tauri v2](https://v2.tauri.app/) và giao diện [Leptos](https://leptos.dev/) (Rust WebAssembly CSR).

---

## 🚀 CI/CD & Đóng gói tự động (GitHub Actions)

Dự án đã được thiết lập sẵn workflow tự động build và đóng gói cho **Windows** và **macOS** tại [`.github/workflows/release.yml`](.github/workflows/release.yml).

### Các nền tảng được hỗ trợ:
- **Windows (x86_64)**: Tạo bộ cài đặt `.exe` (NSIS đa người dùng / người dùng hiện tại) và `.msi`.
- **macOS (Universal Binary)**: Biên dịch cho cả Apple Silicon (`aarch64`) và Intel (`x86_64`), đóng gói định dạng `.dmg` và `.app`.

### Cách tạo bản phát hành (Release):
1. Cập nhật version trong `src-tauri/tauri.conf.json` và `Cargo.toml`.
2. Tạo git tag và push lên GitHub:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. GitHub Actions sẽ tự động biên dịch trên các máy ảo Windows và macOS, sau đó tạo một **Draft Release** trên GitHub chứa đầy đủ các file cài đặt (`.exe`, `.msi`, `.dmg`).

### Build thử nghiệm hoặc thủ công (Manual / PR):
- **Manual Trigger**: Vào tab **Actions** trên GitHub repository -> chọn workflow **Release & Build Matrix** -> bấm **Run workflow**.
- **Tải file cài đặt**: Mỗi lần workflow chạy (kể cả khi chưa gắn tag release), file cài đặt của từng hệ điều hành sẽ được lưu tại mục **Artifacts** ở trang tổng kết của lượt chạy đó.

> [!IMPORTANT]
> **Cấu hình quyền trên GitHub Repository:**
> Để GitHub Actions có quyền tạo Release và upload file, hãy truy cập:
> **Settings > Actions > General > Workflow permissions** -> chọn **Read and write permissions** -> bấm **Save**.

---

## 💻 Cài đặt môi trường phát triển cục bộ (Local Development)

### 1. Dành cho Windows:
1. Cài đặt [Rust](https://www.rust-lang.org/tools/install) (chọn toolchain `x86_64-pc-windows-msvc`).
2. Cài đặt C++ Build Tools qua Visual Studio Installer ("Desktop development with C++").
3. Thêm target WebAssembly:
   ```powershell
   rustup target add wasm32-unknown-unknown
   ```
4. Cài đặt Trunk:
   ```powershell
   cargo install trunk
   ```
5. Cài đặt Tauri CLI (nếu chưa có) và chạy ứng dụng:
   ```powershell
   cargo install tauri-cli --version "^2.0"
   cargo tauri dev
   ```

### 2. Dành cho macOS:
1. Cài đặt Xcode Command Line Tools:
   ```bash
   xcode-select --install
   ```
2. Cài đặt Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. Thêm target WebAssembly:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
4. Cài đặt Trunk:
   ```bash
   brew install trunk
   # hoặc: cargo install trunk
   ```
5. Chạy ứng dụng ở chế độ phát triển:
   ```bash
   cargo install tauri-cli --version "^2.0"
   cargo tauri dev
   ```

### 3. Dành cho Linux:
1. Cài đặt các thư viện hệ thống cần thiết (Ubuntu/Debian):
   ```bash
   sudo apt update
   sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
   ```
2. Thêm target WebAssembly:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. Cài đặt Trunk và chạy dự án:
   ```bash
   cargo install trunk
   cargo install tauri-cli --version "^2.0"
   cargo tauri dev
   ```
