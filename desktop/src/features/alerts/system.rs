//! The operating system's side of the alerts: whether the app can reach the notification center,
//! and the reader's permission (docs/native-apps.md §5.8 "Settings"; iOS `AlertSystem.swift`).
//!
//! Posting itself goes through gpui-kit (`Notification::system()`), which drops posts silently
//! when the app doesn't run from a bundle; `has_notification_center` tells the alerts to show
//! in-app toasts instead. On macOS this asks `UNUserNotificationCenter` directly through the
//! Objective-C runtime (the framework is already linked by GPUI), and only from a bundle: outside
//! one the framework aborts the process.

use std::rc::Rc;

use futures::FutureExt;
use futures::future::LocalBoxFuture;

/// The reader's answer for this app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Not asked yet: turning a switch on asks.
    NotDetermined,
    Denied,
    Allowed,
    /// No notification center (an unbundled build, another platform): alerts show as toasts.
    Unavailable,
}

pub trait AlertSystem {
    /// Posts can reach the notification center. Without it there's no permission to ask, and the
    /// alerts show as in-app toasts.
    fn has_notification_center(&self) -> bool;
    /// The permission as it stands (System Settings can change it at any time).
    fn permission(&self) -> LocalBoxFuture<'static, Permission>;
    /// Asks for permission (macOS shows its prompt once; afterwards this answers at once).
    fn request_permission(&self) -> LocalBoxFuture<'static, bool>;
}

/// This computer's notification center.
pub fn default_system() -> Rc<dyn AlertSystem> {
    #[cfg(debug_assertions)]
    if let Some(system) = debug_system() {
        return system;
    }
    #[cfg(target_os = "macos")]
    {
        Rc::new(macos::MacSystem::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Rc::new(NoNotificationCenter)
    }
}

/// No notification center: toasts only.
pub struct NoNotificationCenter;

impl AlertSystem for NoNotificationCenter {
    fn has_notification_center(&self) -> bool {
        false
    }

    fn permission(&self) -> LocalBoxFuture<'static, Permission> {
        futures::future::ready(Permission::Unavailable).boxed_local()
    }

    fn request_permission(&self) -> LocalBoxFuture<'static, bool> {
        futures::future::ready(true).boxed_local()
    }
}

/// A notification center whose answers are set by hand: tests, and debug builds'
/// `LS_ALERTS_PERMISSION=denied|allowed|ask` (screenshots of the blocked state). It never posts:
/// the alerts show as toasts unless `posts` is set.
pub struct FakeSystem {
    pub permission: std::cell::Cell<Permission>,
    /// What `request_permission` answers (and the permission it leaves behind).
    pub grants: bool,
    pub requests: std::cell::Cell<u32>,
}

impl FakeSystem {
    pub fn new(permission: Permission, grants: bool) -> Rc<Self> {
        Rc::new(Self { permission: std::cell::Cell::new(permission), grants, requests: std::cell::Cell::new(0) })
    }
}

impl AlertSystem for FakeSystem {
    fn has_notification_center(&self) -> bool {
        self.permission.get() != Permission::Unavailable
    }

    fn permission(&self) -> LocalBoxFuture<'static, Permission> {
        futures::future::ready(self.permission.get()).boxed_local()
    }

    fn request_permission(&self) -> LocalBoxFuture<'static, bool> {
        self.requests.set(self.requests.get() + 1);
        let granted = match self.permission.get() {
            Permission::NotDetermined => self.grants,
            permission => permission != Permission::Denied,
        };
        if self.permission.get() == Permission::NotDetermined {
            self.permission.set(if granted { Permission::Allowed } else { Permission::Denied });
        }
        futures::future::ready(granted).boxed_local()
    }
}

#[cfg(debug_assertions)]
fn debug_system() -> Option<Rc<dyn AlertSystem>> {
    let permission = match std::env::var("LS_ALERTS_PERMISSION").ok()?.as_str() {
        "denied" => Permission::Denied,
        "allowed" => Permission::Allowed,
        "ask" => Permission::NotDetermined,
        _ => return None,
    };
    Some(FakeSystem::new(permission, true))
}

/// macOS System Settings › Notifications (macOS 13+).
// copy-exempt: a System Settings URL
pub const SETTINGS_URL: &str = "x-apple.systempreferences:com.apple.Notifications-Settings.extension";

#[cfg(target_os = "macos")]
pub mod macos {
    use std::sync::Mutex;

    use block2::RcBlock;
    use futures::FutureExt;
    use futures::channel::oneshot;
    use futures::future::LocalBoxFuture;
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2_foundation::NSBundle;

    use super::{AlertSystem, Permission};

    /// `UNAuthorizationOptionSound | UNAuthorizationOptionAlert`, as gpui asks for.
    const SOUND_AND_ALERT: usize = (1 << 1) | (1 << 2);

    /// `UNUserNotificationCenter`, when the app runs from a bundle.
    pub struct MacSystem {
        bundled: bool,
    }

    impl MacSystem {
        pub fn new() -> Self {
            Self { bundled: NSBundle::mainBundle().bundleIdentifier().is_some() }
        }

        fn center(&self) -> Option<Retained<AnyObject>> {
            if !self.bundled {
                return None;
            }
            let class = AnyClass::get(c"UNUserNotificationCenter")?;
            // SAFETY: a class method returning the shared center (autoreleased, retained here).
            unsafe { msg_send![class, currentNotificationCenter] }
        }
    }

    impl Default for MacSystem {
        fn default() -> Self {
            Self::new()
        }
    }

    /// A completion handler's one answer, sent from whatever thread it runs on.
    fn once<T: Send + 'static>() -> (Mutex<Option<oneshot::Sender<T>>>, oneshot::Receiver<T>) {
        let (sender, receiver) = oneshot::channel();
        (Mutex::new(Some(sender)), receiver)
    }

    fn send<T>(sender: &Mutex<Option<oneshot::Sender<T>>>, value: T) {
        if let Some(sender) = sender.lock().ok().and_then(|mut sender| sender.take()) {
            let _ = sender.send(value);
        }
    }

    impl AlertSystem for MacSystem {
        fn has_notification_center(&self) -> bool {
            self.bundled
        }

        fn permission(&self) -> LocalBoxFuture<'static, Permission> {
            let Some(center) = self.center() else {
                return futures::future::ready(Permission::Unavailable).boxed_local();
            };
            let (sender, receiver) = once::<isize>();
            let block = RcBlock::new(move |settings: *mut AnyObject| {
                // SAFETY: `settings` is a UNNotificationSettings for the duration of the call.
                let status = unsafe { settings.as_ref() }
                    .map(|settings| unsafe { msg_send![settings, authorizationStatus] })
                    .unwrap_or(0);
                send(&sender, status);
            });
            // SAFETY: the documented selector, with a block of the right signature.
            unsafe {
                let _: () = msg_send![&*center, getNotificationSettingsWithCompletionHandler: &*block];
            }
            async move {
                // UNAuthorizationStatus: 0 not determined, 1 denied, 2 authorized, 3 provisional,
                // 4 ephemeral.
                match receiver.await.unwrap_or(0) {
                    0 => Permission::NotDetermined,
                    1 => Permission::Denied,
                    _ => Permission::Allowed,
                }
            }
            .boxed_local()
        }

        fn request_permission(&self) -> LocalBoxFuture<'static, bool> {
            let Some(center) = self.center() else {
                return futures::future::ready(true).boxed_local();
            };
            let (sender, receiver) = once::<bool>();
            let block = RcBlock::new(move |granted: Bool, error: *mut AnyObject| {
                // SAFETY: when non-null, an NSError for the duration of the call.
                if let Some(error) = unsafe { error.as_ref() } {
                    let description: Option<Retained<objc2_foundation::NSString>> =
                        unsafe { msg_send![error, localizedDescription] };
                    let description = description.map(|text| text.to_string()).unwrap_or_default();
                    log::warn!("notification permission request failed: {description}");
                }
                send(&sender, granted.as_bool());
            });
            // SAFETY: the documented selector, with a block of the right signature.
            unsafe {
                let _: () =
                    msg_send![&*center, requestAuthorizationWithOptions: SOUND_AND_ALERT, completionHandler: &*block];
            }
            async move { receiver.await.unwrap_or(false) }.boxed_local()
        }
    }

    /// Debug builds: the identifiers of this app's notifications in Notification Centre, to check
    /// that a post arrived (logged by the alerts' evidence steps).
    #[cfg(debug_assertions)]
    pub fn delivered_ids(system: &MacSystem) -> LocalBoxFuture<'static, Vec<String>> {
        let Some(center) = system.center() else {
            return futures::future::ready(Vec::new()).boxed_local();
        };
        let (sender, receiver) = once::<Vec<String>>();
        let block = RcBlock::new(move |notifications: *mut AnyObject| {
            let mut ids = Vec::new();
            // SAFETY: an NSArray<UNNotification *> for the duration of the call.
            if let Some(array) = unsafe { notifications.as_ref() } {
                let count: usize = unsafe { msg_send![array, count] };
                for index in 0..count {
                    unsafe {
                        let notification: *mut AnyObject = msg_send![array, objectAtIndex: index];
                        let request: *mut AnyObject = msg_send![notification, request];
                        let identifier: *mut objc2_foundation::NSString = msg_send![request, identifier];
                        if let Some(identifier) = identifier.as_ref() {
                            ids.push(identifier.to_string());
                        }
                    }
                }
            }
            send(&sender, ids);
        });
        // SAFETY: the documented selector, with a block of the right signature.
        unsafe {
            let _: () = msg_send![&*center, getDeliveredNotificationsWithCompletionHandler: &*block];
        }
        async move { receiver.await.unwrap_or_default() }.boxed_local()
    }
}
