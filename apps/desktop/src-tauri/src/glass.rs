//! Native Liquid Glass behind the sidebar on macOS 26+.
//!
//! The web UI measures its sidebar and calls [`set_glass_frame`]; an
//! `NSGlassEffectView` is placed under the web view at that spot, over a
//! native copy of the app's sky gradient, and the UI leaves a hole in its
//! background there so the system glass shows through. The window stays
//! opaque: the glass refracts the app's background, never the desktop. On older macOS, and on Windows, nothing happens and
//! the UI keeps its CSS imitation.

#[cfg(target_os = "macos")]
mod mac {
    use objc2::encode::{Encode, Encoding};
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The glass view and the sky painted behind it, created once on the
    /// main thread and kept for the life of the window (the +1 from
    /// alloc/init is never released).
    static GLASS: AtomicUsize = AtomicUsize::new(0);

    const NS_WINDOW_ABOVE: isize = 1;
    const NS_WINDOW_BELOW: isize = -1;
    const NS_VIEW_MAX_X_MARGIN: usize = 4;
    const NS_VIEW_WIDTH_SIZABLE: usize = 2;
    const NS_VIEW_HEIGHT_SIZABLE: usize = 16;

    /// `CGColorRef`, with the encoding AppKit declares for it (objc2 checks
    /// encodings in debug builds).
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct CgColor(*const std::ffi::c_void);
    // SAFETY: a pointer to the opaque CGColor struct, as in the SDK headers.
    unsafe impl Encode for CgColor {
        const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
    }

    fn glass_class() -> Option<&'static AnyClass> {
        AnyClass::get(c"NSGlassEffectView")
    }

    pub fn supported() -> bool {
        glass_class().is_some()
    }

    unsafe fn class(name: &std::ffi::CStr) -> Option<&'static AnyClass> {
        AnyClass::get(name)
    }

    /// The app's sky (--sky in app.css) as a native gradient filling the
    /// window, so the glass refracts the app's own background and not the
    /// desktop. Colours go bottom to top: the layer is not flipped.
    unsafe fn make_sky(bounds: NSRect) -> Option<*mut AnyObject> {
        let (view_cls, layer_cls) = (class(c"NSView")?, class(c"CAGradientLayer")?);
        let (color_cls, array_cls, number_cls) = (class(c"NSColor")?, class(c"NSMutableArray")?, class(c"NSNumber")?);
        let colors: *mut AnyObject = msg_send![array_cls, array];
        let locations: *mut AnyObject = msg_send![array_cls, array];
        // bottom #33478f, middle #4a72d4 (45% from the top), top #3a86ec
        for ((r, g, b), at) in [((51.0, 71.0, 143.0), 0.0), ((74.0, 114.0, 212.0), 0.55), ((58.0, 134.0, 236.0), 1.0)] {
            let c: *mut AnyObject =
                msg_send![color_cls, colorWithSRGBRed: r / 255.0, green: g / 255.0, blue: b / 255.0, alpha: 1.0_f64];
            let cg: CgColor = msg_send![c, CGColor];
            let _: () = msg_send![colors, addObject: cg.0 as *mut AnyObject];
            let n: *mut AnyObject = msg_send![number_cls, numberWithDouble: at];
            let _: () = msg_send![locations, addObject: n];
        }
        let layer: *mut AnyObject = msg_send![layer_cls, layer];
        let _: () = msg_send![layer, setColors: colors];
        let _: () = msg_send![layer, setLocations: locations];

        let view: *mut AnyObject = msg_send![view_cls, alloc];
        let view: *mut AnyObject = msg_send![view, initWithFrame: bounds];
        if view.is_null() {
            return None;
        }
        // Layer-hosting view: set the layer first, then ask for it.
        let _: () = msg_send![view, setLayer: layer];
        let _: () = msg_send![view, setWantsLayer: true];
        let _: () = msg_send![view, setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE];
        Some(view)
    }

    /// Must run on the main thread. `y` is from the top, as in the web view.
    pub unsafe fn place(ns_window: *mut AnyObject, x: f64, y: f64, w: f64, h: f64, radius: f64) {
        let Some(glass_cls) = glass_class() else { return };
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
            let Some(sky) = make_sky(bounds) else { return };
            let obj: *mut AnyObject = msg_send![glass_cls, alloc];
            let obj: *mut AnyObject = msg_send![obj, initWithFrame: frame];
            if obj.is_null() {
                return;
            }
            // Same top and bottom margins as the sidebar when the window is
            // resized, before the UI reports the new size.
            let mask = NS_VIEW_HEIGHT_SIZABLE | NS_VIEW_MAX_X_MARGIN;
            let _: () = msg_send![obj, setAutoresizingMask: mask];
            let none = std::ptr::null_mut::<AnyObject>();
            // Sky at the very back, glass right above it, the web view on top.
            let _: () = msg_send![content, addSubview: sky, positioned: NS_WINDOW_BELOW, relativeTo: none];
            let _: () = msg_send![content, addSubview: obj, positioned: NS_WINDOW_ABOVE, relativeTo: sky];
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
