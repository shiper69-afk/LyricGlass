//! Desktop background image for wallpaper mode (SPI + optional base64 payload).

#[cfg(windows)]
mod imp {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::Serialize;
    use std::ffi::OsString;
    use std::os::windows::prelude::OsStringExt;
    use windows::Win32::Graphics::Gdi::{GetSysColor, COLOR_DESKTOP};
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DesktopWallpaperPayload {
        pub path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data_url: Option<String>,
        pub desktop_color: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub read_error: Option<String>,
    }

    fn spi_wallpaper_path() -> Result<String, String> {
        let mut buf = vec![0u16; 4096];
        unsafe {
            SystemParametersInfoW(
                SPI_GETDESKWALLPAPER,
                buf.len() as u32,
                Some(buf.as_mut_ptr() as *mut _),
                Default::default(),
            )
            .map_err(|e| format!("SPI_GETDESKWALLPAPER: {e}"))?;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(OsString::from_wide(&buf[..len])
            .to_string_lossy()
            .into_owned())
    }

    fn desktop_color_hex() -> String {
        unsafe {
            let c = GetSysColor(COLOR_DESKTOP) as u32;
            let r = c & 0xFF;
            let g = (c >> 8) & 0xFF;
            let b = (c >> 16) & 0xFF;
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    pub fn desktop_wallpaper_payload() -> DesktopWallpaperPayload {
        let desktop_color = desktop_color_hex();
        let path = match spi_wallpaper_path() {
            Ok(p) => p,
            Err(e) => {
                crate::startup::step(&format!("desktop wallpaper SPI failed: {e}"));
                return DesktopWallpaperPayload {
                    path: String::new(),
                    data_url: None,
                    desktop_color,
                    read_error: Some(e),
                };
            }
        };
        crate::startup::step(&format!("desktop wallpaper path resolved: {path}"));

        let mut read_error = None;
        let data_url = if path.is_empty() {
            None
        } else {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let mime = match std::path::Path::new(&path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase())
                        .as_deref()
                    {
                        Some("png") => "image/png",
                        Some("jpg") | Some("jpeg") => "image/jpeg",
                        Some("bmp") => "image/bmp",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        _ => "image/jpeg",
                    };
                    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
                }
                Err(e) => {
                    read_error = Some(format!("read {path}: {e}"));
                    crate::startup::step(&format!(
                        "desktop wallpaper read failed: {read_error:?}"
                    ));
                    None
                }
            }
        };

        DesktopWallpaperPayload {
            path,
            data_url,
            desktop_color,
            read_error,
        }
    }
}

#[cfg(windows)]
pub use imp::{desktop_wallpaper_payload, DesktopWallpaperPayload};

#[cfg(not(windows))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWallpaperPayload {
    pub path: String,
    pub data_url: Option<String>,
    pub desktop_color: String,
    pub read_error: Option<String>,
}

#[cfg(not(windows))]
pub fn desktop_wallpaper_payload() -> DesktopWallpaperPayload {
    DesktopWallpaperPayload {
        path: String::new(),
        data_url: None,
        desktop_color: "#1a1a2e".into(),
        read_error: Some("Windows only".into()),
    }
}