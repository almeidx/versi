#[cfg(target_os = "macos")]
pub(super) fn set_update_badge(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let tile = app.dockTile();
    if visible {
        tile.setBadgeLabel(Some(&NSString::from_str("1")));
    } else {
        tile.setBadgeLabel(None);
    }
}

#[cfg(target_os = "linux")]
pub(super) fn set_update_badge(visible: bool) {
    use log::debug;

    std::thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let connection = zbus::blocking::Connection::session()?;

            let count: i64 = if visible { 1 } else { 0 };
            let mut props = std::collections::HashMap::new();
            props.insert("count", zbus::zvariant::Value::from(count));
            props.insert("count-visible", zbus::zvariant::Value::from(visible));

            connection.emit_signal(
                None::<zbus::names::BusName>,
                "/",
                "com.canonical.Unity.LauncherEntry",
                "Update",
                &("application://dev.almeidx.versi.desktop", props),
            )?;

            Ok(())
        })();

        if let Err(e) = result {
            debug!("Failed to set update badge: {}", e);
        }
    });
}

#[cfg(windows)]
pub(super) fn set_update_badge(visible: bool) {
    use std::ptr;

    use log::debug;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize,
    };
    use windows::Win32::UI::Shell::ITaskbarList3;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateIconIndirect, DestroyIcon, FindWindowA, HICON, ICONINFO,
    };
    use windows::core::{s, w, PCSTR, PCWSTR};

    unsafe {
        let hwnd = match FindWindowA(PCSTR::null(), s!("Versi")) {
            Ok(h) if h.is_invalid() => {
                debug!("Could not find Versi window for badge");
                return;
            }
            Ok(h) => h,
            Err(_) => {
                debug!("Could not find Versi window for badge");
                return;
            }
        };

        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let taskbar: ITaskbarList3 = CoCreateInstance(
                &windows::Win32::UI::Shell::TaskbarList,
                None,
                CLSCTX_INPROC_SERVER,
            )?;

            if !visible {
                taskbar.SetOverlayIcon(hwnd, HICON::default(), PCWSTR::null())?;
                return Ok(());
            }

            // Create a 16x16 red circle icon
            let size: i32 = 16;
            let mut pixels = vec![0u8; (size * size * 4) as usize];

            let center = size as f32 / 2.0;
            let radius = center - 1.0;

            for y in 0..size {
                for x in 0..size {
                    let dx = x as f32 - center + 0.5;
                    let dy = y as f32 - center + 0.5;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let offset = ((y * size + x) * 4) as usize;

                    if dist <= radius {
                        // BGRA format: red circle
                        pixels[offset] = 0x33; // B
                        pixels[offset + 1] = 0x33; // G
                        pixels[offset + 2] = 0xEE; // R
                        pixels[offset + 3] = 0xFF; // A
                    }
                }
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: size,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let dc = CreateCompatibleDC(None);
            let mut bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
            let color_bitmap = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0)?;

            ptr::copy_nonoverlapping(pixels.as_ptr(), bits_ptr as *mut u8, pixels.len());

            // Create mask bitmap (all zeros = fully opaque)
            let mask_bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: size,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut mask_bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
            let mask_bitmap =
                CreateDIBSection(Some(dc), &mask_bmi, DIB_RGB_COLORS, &mut mask_bits_ptr, None, 0)?;

            ptr::write_bytes(mask_bits_ptr as *mut u8, 0, pixels.len());

            let icon_info = ICONINFO {
                fIcon: true.into(),
                xHotspot: 0,
                yHotspot: 0,
                hbmMask: mask_bitmap,
                hbmColor: color_bitmap,
            };

            let icon = CreateIconIndirect(&icon_info)?;
            let result = taskbar.SetOverlayIcon(hwnd, icon, w!("Update available"));

            let _ = DestroyIcon(icon);
            let _ = DeleteObject(color_bitmap.into());
            let _ = DeleteObject(mask_bitmap.into());
            let _ = DeleteDC(dc);

            result?;
            Ok(())
        })();

        CoUninitialize();

        if let Err(e) = result {
            debug!("Failed to set update badge: {}", e);
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub(super) fn set_update_badge(_visible: bool) {}

#[cfg(target_os = "macos")]
pub(super) fn set_dock_visible(visible: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    app.setActivationPolicy(policy);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn set_dock_visible(_visible: bool) {}

#[cfg(target_os = "linux")]
pub(super) fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "wayland")
        .unwrap_or_else(|_| std::env::var("WAYLAND_DISPLAY").is_ok())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn is_wayland() -> bool {
    false
}

pub(super) fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        use versi_core::HideWindow;
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .hide_window()
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}
