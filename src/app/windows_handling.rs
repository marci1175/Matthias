use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONEXCLAMATION};
use windows_sys::w;

pub fn win_err_exclamation(title :  &str, text : &str) {
    std::thread::spawn(|| {
        unsafe
        {
            MessageBoxW(0, caption, w!(title), MB_ICONEXCLAMATION);
        }
    });
} 
    