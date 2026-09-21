//! Native macOS notifications through `UNUserNotificationCenter`: a real
//! authorization prompt, a real status, banners while the app is in the
//! foreground, and click handling that routes back to the task.
//!
//! This only works for a bundled app: a raw `tauri dev` binary has no bundle
//! identifier, so the whole module no-ops there and the caller falls back to
//! the legacy notification path.

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, ClassType, DefinedClass};
use objc2_foundation::{NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotification,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNNotificationSettings, UNNotificationSound, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use tauri::{Emitter, Manager};

struct DelegateIvars {
    app: tauri::AppHandle,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TackNotificationDelegate"]
    #[ivars = DelegateIvars]
    struct Delegate;

    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            // a foreground app would otherwise swallow the banner
            completion_handler.call((
                UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,
            ));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &DynBlock<dyn Fn()>,
        ) {
            // the request identifier carries the task id
            let task_id = response.notification().request().identifier().to_string();
            if !task_id.is_empty() {
                let app = self.ivars().app.clone();
                let app_for_main = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(window) = app_for_main.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                });
                let _ = app.emit("reminder-clicked", task_id);
            }
            completion_handler.call(());
        }
    }

    unsafe impl NSObjectProtocol for Delegate {}
);

impl Delegate {
    fn new(app: tauri::AppHandle) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars { app });
        unsafe { msg_send![super(this), init] }
    }
}

// only a real .app bundle has a bundle proxy, which the notification center
// requires; bundleIdentifier() alone is non-nil even for a bare binary, so we
// check the bundle path instead. getting this wrong terminates the process
pub fn is_bundled() -> bool {
    NSBundle::mainBundle().bundlePath().to_string().ends_with(".app")
}

// register the delegate so foreground banners and clicks reach us
pub fn install(app: &tauri::AppHandle) {
    if !is_bundled() {
        return;
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let delegate = Delegate::new(app.clone());
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // the center keeps the delegate weakly; hold ours for the app lifetime
    std::mem::forget(delegate);
}

pub fn status() -> &'static str {
    if !is_bundled() {
        return "unsupported";
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        let value = unsafe { settings.as_ref() }.authorizationStatus().0;
        let _ = tx.send(value);
    });
    center.getNotificationSettingsWithCompletionHandler(&handler);
    let authorized = UNAuthorizationStatus::Authorized.0;
    let denied = UNAuthorizationStatus::Denied.0;
    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(value) if value == authorized => "granted",
        Ok(value) if value == denied => "denied",
        _ => "prompt",
    }
}

pub fn request() {
    if !is_bundled() {
        return;
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let options = UNAuthorizationOptions::Alert
        | UNAuthorizationOptions::Sound
        | UNAuthorizationOptions::Badge;
    let handler = RcBlock::new(|_granted: Bool, _error: *mut NSError| {});
    center.requestAuthorizationWithOptions_completionHandler(options, &handler);
}

pub fn send(task_id: &str, title: &str, subtitle: Option<&str>, body: &str) {
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    if let Some(subtitle) = subtitle {
        content.setSubtitle(&NSString::from_str(subtitle));
    }
    content.setBody(&NSString::from_str(body));
    content.setSound(Some(&UNNotificationSound::defaultSound()));
    // no trigger = deliver now; the identifier is the task id we route on click
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
        &NSString::from_str(task_id),
        content.as_super(),
        None,
    );
    center.addNotificationRequest_withCompletionHandler(&request, None);
}
