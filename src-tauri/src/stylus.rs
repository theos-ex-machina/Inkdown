//! Native stylus capture for Linux.
//!
//! WebKitGTK collapses tablet tools into plain mouse events, so the DOM never
//! sees `pointerType == "pen"`, real pressure or tilt (tauri-apps/tauri#10636).
//! Instead we attach a `GtkGestureStylus` to the webview widget in the GTK
//! capture phase. Pen sequences that land inside the ink canvas (a region the
//! frontend reports via `set_stylus_region`) are claimed — WebKit never sees
//! them, so no phantom mouse clicks — and streamed to the frontend as
//! `stylus` events. Pen input outside the canvas is denied and falls through
//! to WebKit as ordinary mouse input, so the pen can still click buttons.
//!
//! On non-Linux targets this module is a no-op: Windows/macOS WebViews report
//! `pointerType == "pen"` natively and the frontend handles it in the DOM.

use std::sync::Mutex;

/// Ink canvas bounding box in CSS pixels, relative to the webview viewport.
/// GTK3 logical pixels match CSS pixels at zoom 1, so no conversion needed.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Region {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

#[derive(Default)]
pub struct StylusState {
    /// `None` while no ink canvas is mounted; the pen then acts as a mouse.
    region: Mutex<Option<Region>>,
}

#[tauri::command]
pub fn set_stylus_region(state: tauri::State<'_, StylusState>, region: Option<Region>) {
    *state.region.lock().unwrap() = region;
}

#[cfg(target_os = "linux")]
mod imp {
    use super::StylusState;
    use gtk::prelude::*;
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Emitter, Manager};

    #[derive(Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StylusPayload {
        phase: &'static str,
        x: f64,
        y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        eraser: bool,
    }

    fn emit_stylus(handle: &AppHandle, g: &gtk::GestureStylus, phase: &'static str, x: f64, y: f64) {
        let payload = StylusPayload {
            phase,
            x,
            y,
            pressure: g.axis(gdk::AxisUse::Pressure).unwrap_or(0.5),
            tilt_x: g.axis(gdk::AxisUse::Xtilt).unwrap_or(0.0),
            tilt_y: g.axis(gdk::AxisUse::Ytilt).unwrap_or(0.0),
            eraser: g
                .device_tool()
                .is_some_and(|t| t.tool_type() == gdk::DeviceToolType::Eraser),
        };
        let _ = handle.emit("stylus", payload);
    }

    fn in_region(handle: &AppHandle, x: f64, y: f64) -> bool {
        let state = handle.state::<StylusState>();
        let region = state.region.lock().unwrap();
        region.is_some_and(|r| r.contains(x, y))
    }

    pub fn attach(app: &tauri::App) {
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let handle = app.handle().clone();

        // The closure runs on the GTK main thread, which is required for
        // touching GTK objects. Signal handlers below also run there.
        let result = window.with_webview(move |platform_webview| {
            let webview = platform_webview.inner();
            let gesture = gtk::GestureStylus::new(&webview);
            // Capture phase: see tablet events before WebKit's own handlers.
            gesture.set_propagation_phase(gtk::PropagationPhase::Capture);

            let h = handle.clone();
            // Hover motion fires continuously; throttle to spare the IPC bus.
            let last_proximity = Cell::new(Instant::now() - Duration::from_secs(1));
            gesture.connect_proximity(move |g, x, y| {
                if last_proximity.get().elapsed() >= Duration::from_millis(100) {
                    last_proximity.set(Instant::now());
                    emit_stylus(&h, g, "proximity", x, y);
                }
            });

            let h = handle.clone();
            gesture.connect_down(move |g, x, y| {
                if in_region(&h, x, y) {
                    // Swallow the sequence so WebKit never synthesizes a
                    // mouse click from this pen contact.
                    g.set_state(gtk::EventSequenceState::Claimed);
                    emit_stylus(&h, g, "down", x, y);
                } else {
                    // Outside the canvas the pen acts as a mouse (toolbar,
                    // sidebar). Still announce it so the app can prep ink mode.
                    emit_stylus(&h, g, "proximity", x, y);
                    g.set_state(gtk::EventSequenceState::Denied);
                }
            });

            let h = handle.clone();
            gesture.connect_motion(move |g, x, y| {
                emit_stylus(&h, g, "motion", x, y);
            });

            let h = handle.clone();
            gesture.connect_up(move |g, x, y| {
                emit_stylus(&h, g, "up", x, y);
            });

            // GTK3 widgets don't own their event controllers (unlike GTK4);
            // leak one reference so the gesture lives as long as the webview.
            std::mem::forget(gesture);
        });

        if let Err(err) = result {
            eprintln!("[stylus] failed to attach native stylus capture: {err}");
        }
    }
}

#[cfg(target_os = "linux")]
pub use imp::attach;

#[cfg(not(target_os = "linux"))]
pub fn attach(_app: &tauri::App) {}
