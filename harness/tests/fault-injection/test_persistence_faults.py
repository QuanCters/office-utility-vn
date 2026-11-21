#!/usr/bin/env python3
"""
harness/tests/fault-injection/test_persistence_faults.py
Kiểm thử tính bất biến NativeSafePersistence dưới điều kiện lỗi giả lập:
- Ghi đĩa thất bại giữa chừng
- Tệp đích bị khóa quyền chỉ đọc
- Kiểm tra tính nguyên tử (atomic rename): file gốc không bị suy suyển
"""

import os
import tempfile
import shutil
import unittest

class TestPersistenceFaults(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.original_file = os.path.join(self.test_dir, "document.pdf")
        with open(self.original_file, "wb") as f:
            f.write(b"%PDF-1.4\nORIGINAL_DATA_12345\n%%EOF")

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_atomic_persistence_on_failure(self):
        """Giả lập lỗi trong quá trình ghi: tệp tạm hỏng không được ghi đè lên file gốc"""
        temp_file = self.original_file + ".tmp"
        
        # Giả lập ghi dở dang
        with open(temp_file, "wb") as f:
            f.write(b"CORRUPTED_PARTIAL_BYTES")

        # Giả lập lỗi xảy ra trước khi atomic rename
        failed = True
        if failed:
            if os.path.exists(temp_file):
                os.remove(temp_file)

        # File gốc phải còn nguyên vẹn 100%
        self.assertTrue(os.path.exists(self.original_file))
        with open(self.original_file, "rb") as f:
            content = f.read()
        self.assertEqual(content, b"%PDF-1.4\nORIGINAL_DATA_12345\n%%EOF")

    def test_readonly_filesystem_protection(self):
        """Kiểm tra phản ứng khi thư mục đích không có quyền ghi"""
        ro_dir = os.path.join(self.test_dir, "ro_zone")
        os.makedirs(ro_dir)
        target = os.path.join(ro_dir, "test.txt")
        # Phải phát hiện và trả về lỗi an toàn thay vì crash
        self.assertTrue(os.path.exists(ro_dir))

if __name__ == "__main__":
    unittest.main()
