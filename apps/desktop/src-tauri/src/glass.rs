//! Native Liquid Glass behind the sidebar on macOS 26+.
//!
//! The web UI measures its sidebar and calls [`set_glass_frame`]; an
//! `NSGlassEffectView` is placed under the (transparent) web view at that
//! spot, and the UI leaves a hole in its background there so the system
//! glass shows through. On older macOS, and on Windows, nothing happens and
//! the UI keeps its CSS imitation.

#[cfg(target_os = "macos")]
mod mac {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The glass view, created once on the main thread and kept for the
    /// life of the window (the +1 from alloc/init is never released).
    static GLASS: AtomicUsize = AtomicUsize::new(0);

    const NS_WINDOW_BELOW: isize = -1;
    const NS_VIEW_MAX_X_MARGIN: usize = 4;
    const NS_VIEW_HEIGHT_SIZABLE: usize = 16;

    fn glass_class() -> Option<&'static AnyClass> {
        AnyClass::get(c"NSGlassEffectView")
    }

    pub fn supported() -> bool {
        glass_class().is_some()
    }

    /// Must run on the main thread. `y` is from the top, as in the web view.
    pub unsafe fn place(ns_window: *mut AnyObject, x: f64, y: f64, w: f64, h: f64, radius: f64) {
        let Some(class) = glass_class() else { return };
        let content: *mut AnyObject = msg_send![ns_window, contentView];
        if content.is_null() {
            return;
        }
        let bounds: NSRect = msg_send![content, bounds];
        let flipped: bool = msg_send![content, isFlipped];
        let oy = if flipped { y } else { bounds.size.height - y - h };
        let frame = NSRect::new(NSPoint::new(x, oy), NSSize::new(w, h));

        let mut glass = GLASS.load(Ordering::Relaxed) as *mut AnyObject;
        if glass.is_null() {
            let obj: *mut AnyObject = msg_send![class, alloc];
            let obj: *mut AnyObject = msg_send![obj, initWithFrame: frame];
            if obj.is_null() {
                return;
            }
            // Same top and bottom margins as the sidebar when the window is
            // resized, before the UI reports the new size.
            let mask = NS_VIEW_HEIGHT_SIZABLE | NS_VIEW_MAX_X_MARGIN;
            let _: () = msg_send![obj, setAutoresizingMask: mask];
            let _: () = msg_send![content, addSubview: obj, positioned: NS_WINDOW_BELOW, relativeTo: std::ptr::null_mut::<AnyObject>()];
            GLASS.store(obj as usize, Ordering::Relaxed);
            glass = obj;
        }
        let _: () = msg_send![glass, setFrame: frame];
        let _: () = msg_send![glass, setCornerRadius: radius];
    }
}

/// Places the native glass under the sidebar. Returns whether the system
/// has it (macOS 26+); `false` tells the UI to keep its CSS look.
#[tauri::command]
#[allow(unused_variables)]
pub fn set_glass_frame(window: tauri::WebviewWindow, x: f64, y: f64, width: f64, height: f64, radius: f64) -> bool {
    #[cfg(target_os = "macos")]
    {
        if !mac::supported() {
            return false;
        }
        let Ok(ns_window) = window.ns_window() else { return false };
        let ns_window = ns_window as usize;
        window
            .run_on_main_thread(move || unsafe {
                mac::place(ns_window as *mut objc2::runtime::AnyObject, x, y, width, height, radius)
            })
            .is_ok()
    }
    #[cfg(not(target_os = "macos"))]
    false
}
