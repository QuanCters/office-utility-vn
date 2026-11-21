#!/usr/bin/env python3
import time
import subprocess
import Xlib.display
import Xlib.X
import Xlib.XK
import Xlib.protocol.event
from Xlib.ext import xtest

class GuiDriver:
    def __init__(self, display_str=":0"):
        self.display_str = display_str
        self.display = Xlib.display.Display(display_str)
        self.root = self.display.screen().root

    def find_window(self, name="office-utility-vn"):
        def search(w):
            w_name = w.get_wm_name()
            if w_name and name.lower() in w_name.lower():
                # check size
                geom = w.get_geometry()
                if geom.width > 100 and geom.height > 100:
                    return w
            for child in w.query_tree().children:
                res = search(child)
                if res:
                    return res
            return None
        return search(self.root)

    def get_window_id(self):
        w = self.find_window()
        return hex(w.id) if w else None

    def screenshot(self, path):
        w = self.find_window()
        if not w:
            try:
                subprocess.run(["import", "-window", "root", path], check=True)
            except Exception as e:
                print(f"[Screenshot] Root screenshot failed: {e}")
                return False
        else:
            try:
                subprocess.run(["import", "-window", hex(w.id), path], check=True)
            except Exception as e:
                print(f"[Screenshot] Window screenshot failed: {e}")
                return False
        print(f"[Screenshot] Saved to {path}")
        return True

    def click(self, rel_x, rel_y):
        w = self.find_window()
        if not w:
            raise RuntimeError("Window not found")
        # calculate root origin of window
        root_origin = self.root.translate_coords(w, 0, 0)
        root_x = root_origin.x + rel_x
        root_y = root_origin.y + rel_y
        print(f"[Click] Rel({rel_x}, {rel_y}) -> Root({root_x}, {root_y})")
        
        # move pointer
        xtest.fake_input(self.display, Xlib.X.MotionNotify, x=root_x, y=root_y)
        self.display.sync()
        time.sleep(0.05)
        # press button 1
        xtest.fake_input(self.display, Xlib.X.ButtonPress, 1)
        self.display.sync()
        time.sleep(0.05)
        # release button 1
        xtest.fake_input(self.display, Xlib.X.ButtonRelease, 1)
        self.display.sync()
        time.sleep(0.2)

    def double_click(self, rel_x, rel_y):
        self.click(rel_x, rel_y)
        time.sleep(0.1)
        self.click(rel_x, rel_y)

    def type_text(self, text):
        shift_kc = 50
        for char in text:
            needs_shift = char.isupper() or char in "!@#$%^&*()_+{}|:\"<>?~"
            if char == '_':
                char_to_sym = '-'
            else:
                char_to_sym = char.lower() if needs_shift else char
            keysym = ord(char_to_sym)
            keycode = self.display.keysym_to_keycode(keysym)
            if keycode:
                if needs_shift:
                    xtest.fake_input(self.display, Xlib.X.KeyPress, shift_kc)
                    self.display.sync()
                    time.sleep(0.02)
                xtest.fake_input(self.display, Xlib.X.KeyPress, keycode)
                self.display.sync()
                time.sleep(0.02)
                xtest.fake_input(self.display, Xlib.X.KeyRelease, keycode)
                self.display.sync()
                if needs_shift:
                    time.sleep(0.02)
                    xtest.fake_input(self.display, Xlib.X.KeyRelease, shift_kc)
                    self.display.sync()
                time.sleep(0.05)

    def send_key(self, key_name):
        ks = Xlib.XK.string_to_keysym(key_name)
        kc = self.display.keysym_to_keycode(ks)
        if kc:
            print(f"[Key] Sending {key_name} (keycode {kc})")
            xtest.fake_input(self.display, Xlib.X.KeyPress, kc)
            self.display.sync()
            time.sleep(0.05)
            xtest.fake_input(self.display, Xlib.X.KeyRelease, kc)
            self.display.sync()
            time.sleep(0.1)

    def send_ctrl_key(self, key_char):
        ctrl_kc = self.display.keysym_to_keycode(Xlib.XK.string_to_keysym("Control_L"))
        char_kc = self.display.keysym_to_keycode(ord(key_char))
        print(f"[Key] Sending Ctrl+{key_char}")
        xtest.fake_input(self.display, Xlib.X.KeyPress, ctrl_kc)
        self.display.sync()
        time.sleep(0.05)
        xtest.fake_input(self.display, Xlib.X.KeyPress, char_kc)
        self.display.sync()
        time.sleep(0.05)
        xtest.fake_input(self.display, Xlib.X.KeyRelease, char_kc)
        self.display.sync()
        time.sleep(0.05)
        xtest.fake_input(self.display, Xlib.X.KeyRelease, ctrl_kc)
        self.display.sync()
        time.sleep(0.1)

    def send_alt_f4(self):
        w = self.find_window()
        if not w:
            raise RuntimeError("Window not found")
        # Alt keycode
        alt_kc = self.display.keysym_to_keycode(Xlib.XK.string_to_keysym("Alt_L"))
        f4_kc = self.display.keysym_to_keycode(Xlib.XK.string_to_keysym("F4"))
        print("[Key] Sending Alt+F4")
        xtest.fake_input(self.display, Xlib.X.KeyPress, alt_kc)
        self.display.sync()
        xtest.fake_input(self.display, Xlib.X.KeyPress, f4_kc)
        self.display.sync()
        time.sleep(0.05)
        xtest.fake_input(self.display, Xlib.X.KeyRelease, f4_kc)
        self.display.sync()
        xtest.fake_input(self.display, Xlib.X.KeyRelease, alt_kc)
        self.display.sync()

    def send_wm_delete_window(self):
        w = self.find_window()
        if not w:
            raise RuntimeError("Window not found")
        wm_protocols = self.display.intern_atom('WM_PROTOCOLS')
        wm_delete_window = self.display.intern_atom('WM_DELETE_WINDOW')

        cm_event = Xlib.protocol.event.ClientMessage(
            window=w,
            client_type=wm_protocols,
            data=(32, [wm_delete_window, Xlib.X.CurrentTime, 0, 0, 0])
        )
        print(f"[ICCCM] Sending WM_DELETE_WINDOW to window {hex(w.id)}")
        w.send_event(cm_event, event_mask=Xlib.X.NoEventMask)
        self.display.sync()

if __name__ == "__main__":
    driver = GuiDriver()
    w_id = driver.get_window_id()
    print("Found window ID:", w_id)
    if w_id:
        driver.screenshot("/tmp/office-utility-uat/test_driver_snap.png")
