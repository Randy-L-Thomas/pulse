//! Window pop, enumeration, capture, and WinRT OCR (Windows).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: u64,
}

#[cfg(not(windows))]
pub fn pop_cursor() -> Result<String, String> {
    Err("pop is Windows-only".into())
}

#[cfg(not(windows))]
pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    Err("Windows-only".into())
}

#[cfg(not(windows))]
pub fn capture_and_ocr(title_substr: &str) -> Result<String, String> {
    let _ = title_substr;
    Err("OCR is Windows-only".into())
}

#[cfg(windows)]
mod imp {
    use super::WindowInfo;
    use std::mem::size_of;
    use windows::core::BOOL;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
        SRCCOPY, BI_RGB,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    struct EnumData {
        out: Vec<WindowInfo>,
        needle: Option<String>,
        cursor_hwnds: Vec<HWND>,
    }

    pub fn pop_cursor() -> Result<String, String> {
        let mut data = EnumData {
            out: Vec::new(),
            needle: None,
            cursor_hwnds: Vec::new(),
        };
        unsafe {
            EnumWindows(Some(enum_proc), LPARAM(&mut data as *mut _ as isize))
                .map_err(|e| e.to_string())?;
        }
        let hwnd = data
            .cursor_hwnds
            .into_iter()
            .next()
            .ok_or("no Cursor window")?;
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        Ok("popped Cursor".into())
    }

    pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
        let mut data = EnumData {
            out: Vec::new(),
            needle: None,
            cursor_hwnds: Vec::new(),
        };
        unsafe {
            EnumWindows(Some(enum_proc), LPARAM(&mut data as *mut _ as isize))
                .map_err(|e| e.to_string())?;
        }
        Ok(data.out)
    }

    pub fn capture_and_ocr(title_substr: &str) -> Result<String, String> {
        let needle = title_substr.trim();
        if needle.is_empty() {
            return Err("no window title".into());
        }
        let mut data = EnumData {
            out: Vec::new(),
            needle: Some(needle.to_ascii_lowercase()),
            cursor_hwnds: Vec::new(),
        };
        unsafe {
            EnumWindows(Some(enum_proc), LPARAM(&mut data as *mut _ as isize))
                .map_err(|e| e.to_string())?;
        }
        let hwnd = data
            .out
            .first()
            .map(|w| HWND(w.hwnd as usize as *mut core::ffi::c_void))
            .ok_or_else(|| format!("no window matching '{needle}'"))?;
        let bgra = unsafe { grab_bgra(hwnd) }?;
        ocr_bgra(bgra.0, bgra.1, bgra.2)
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut EnumData);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buf[..n as usize]);
        if title.is_empty() {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let lower = title.to_ascii_lowercase();
        if lower.contains("cursor") && !lower.contains("pulse") {
            data.cursor_hwnds.push(hwnd);
        }
        if let Some(needle) = &data.needle {
            if lower.contains(needle) {
                data.out.push(WindowInfo {
                    title: title.clone(),
                    hwnd: hwnd.0 as usize as u64,
                });
            }
        } else if title.len() > 1 {
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            if rc.right - rc.left >= 120 && rc.bottom - rc.top >= 80 {
                data.out.push(WindowInfo {
                    title,
                    hwnd: hwnd.0 as usize as u64,
                });
            }
        }
        BOOL(1)
    }

    unsafe fn grab_bgra(hwnd: HWND) -> Result<(Vec<u8>, u32, u32), String> {
        let mut rc = RECT::default();
        GetClientRect(hwnd, &mut rc).map_err(|e| e.to_string())?;
        let w = (rc.right - rc.left).max(1) as i32;
        let h = (rc.bottom - rc.top).max(1) as i32;
        let hdc = GetDC(Some(hwnd));
        if hdc.0.is_null() {
            return Err("GetDC failed".into());
        }
        let mem = CreateCompatibleDC(Some(hdc));
        let bmp = CreateCompatibleBitmap(hdc, w, h);
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let _ = BitBlt(mem, 0, 0, w, h, Some(hdc), 0, 0, SRCCOPY);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        ReleaseDC(Some(hwnd), hdc);
        Ok((pixels, w as u32, h as u32))
    }

    fn ocr_bgra(bgra: Vec<u8>, w: u32, h: u32) -> Result<String, String> {
        let bmp_bytes = bgra_to_bmp32(&bgra, w, h);
        let stream = InMemoryRandomAccessStream::new().map_err(|e| e.to_string())?;
        let writer = DataWriter::CreateDataWriter(&stream).map_err(|e| e.to_string())?;
        writer.WriteBytes(&bmp_bytes).map_err(|e| e.to_string())?;
        writer
            .StoreAsync()
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        writer.FlushAsync().map_err(|e| e.to_string())?.get().ok();
        stream.Seek(0).map_err(|e| e.to_string())?;
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        let software = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|e| e.to_string())?;
        let result = engine
            .RecognizeAsync(&software)
            .map_err(|e| e.to_string())?
            .get()
            .map_err(|e| e.to_string())?;
        let text = result.Text().map_err(|e| e.to_string())?;
        Ok(text.to_string())
    }

    fn bgra_to_bmp32(bgra: &[u8], w: u32, h: u32) -> Vec<u8> {
        let row = w * 4;
        let pixel_size = (row * h) as usize;
        let file_size = 54 + pixel_size;
        let mut out = Vec::with_capacity(file_size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&(-(h as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(pixel_size as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(bgra);
        out
    }
}

#[cfg(windows)]
pub use imp::{capture_and_ocr, list_windows, pop_cursor};
