use super::Result;

#[cfg(target_os = "macos")]
pub fn capture(app: &tauri::AppHandle) -> Result<Vec<u8>> {
    macos::capture(app)
}

#[cfg(not(target_os = "macos"))]
pub fn capture(_app: &tauri::AppHandle) -> Result<Vec<u8>> {
    Err("window screenshots are only implemented on macOS".to_string())
}

#[cfg(target_os = "macos")]
mod macos {
    use super::Result;
    use block2::RcBlock;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::msg_send;
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::Manager;

    // -[WKWebView takeSnapshotWithConfiguration:completionHandler:] renders
    // the page straight off the webview, so it needs no screen-recording
    // permission (unlike CGWindowList), works while the window is hidden or
    // occluded, and returns just the ui - no native chrome in the way.
    // with_webview runs the capture on the platform thread as required by
    // webkit; the completion block reports back through the channel
    pub fn capture(app: &tauri::AppHandle) -> Result<Vec<u8>> {
        let webview = app
            .get_webview_window("main")
            .ok_or_else(|| "main window is not available".to_string())?;
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>>>();
        webview
            .with_webview(move |w| {
                let wkwebview = w.inner() as *mut AnyObject;
                if wkwebview.is_null() {
                    let _ = tx.send(Err("no native webview handle".to_string()));
                    return;
                }
                let tx = tx.clone();
                let block = RcBlock::new(
                    move |image: *mut AnyObject, _error: *mut AnyObject| {
                        let _ = tx.send(png_bytes(image));
                    },
                );
                let () = unsafe {
                    msg_send![
                        wkwebview,
                        takeSnapshotWithConfiguration: std::ptr::null::<AnyObject>(),
                        completionHandler: &*block
                    ]
                };
            })
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "webview snapshot timed out".to_string())?
    }

    // NSImage -> PNG bytes via NSBitmapImageRep
    fn png_bytes(image: *mut AnyObject) -> Result<Vec<u8>> {
        if image.is_null() {
            return Err("webview snapshot returned no image".to_string());
        }
        let rep_class = AnyClass::get(c"NSBitmapImageRep")
            .ok_or_else(|| "NSBitmapImageRep unavailable".to_string())?;
        unsafe {
            let tiff: *mut AnyObject = msg_send![image, TIFFRepresentation];
            if tiff.is_null() {
                return Err("snapshot has no image data".to_string());
            }
            let rep: *mut AnyObject = msg_send![rep_class, alloc];
            let rep: *mut AnyObject = msg_send![rep, initWithData: tiff];
            if rep.is_null() {
                return Err("could not decode snapshot bitmap".to_string());
            }
            // NSBitmapImageFileTypePNG = 4
            let png: *mut AnyObject = msg_send![
                rep,
                representationUsingType: 4usize,
                properties: std::ptr::null::<AnyObject>()
            ];
            if png.is_null() {
                return Err("png encode failed".to_string());
            }
            let len: usize = msg_send![png, length];
            let ptr: *const u8 = msg_send![png, bytes];
            Ok(std::slice::from_raw_parts(ptr, len).to_vec())
        }
    }
}
