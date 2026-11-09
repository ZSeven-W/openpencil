//! Keep standard AppKit editing actions available to native controls.
//!
//! Native file panels inspect menu selectors (including across their remote
//! view boundary). A custom menu target cannot replace that responder routing.
//! The canvas participates through a responder after its winit content view.

use std::sync::{mpsc, Mutex, OnceLock};

use objc2::{
    class, declare_class, msg_send_id, mutability, rc::Retained, runtime::AnyObject, ClassType,
    DeclaredClass,
};
use objc2_app_kit::{NSApplication, NSResponder, NSView};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};

use super::MenuAction;

struct Events {
    sender: mpsc::Sender<MenuAction>,
    receiver: Mutex<mpsc::Receiver<MenuAction>>,
}

static EVENTS: OnceLock<Events> = OnceLock::new();
static WAKE_PROXY: OnceLock<winit::event_loop::EventLoopProxy<crate::DesktopEvent>> =
    OnceLock::new();

pub(crate) fn set_wake_proxy(proxy: winit::event_loop::EventLoopProxy<crate::DesktopEvent>) {
    let _ = WAKE_PROXY.set(proxy);
}

pub(super) fn install(resolve: fn(&str) -> Option<MenuAction>) {
    EVENTS.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if let Some(action) = resolve(event.id.as_ref()) {
                enqueue(action);
            }
        }));
        Events {
            sender,
            receiver: Mutex::new(receiver),
        }
    });
}

pub(super) fn poll() -> Option<MenuAction> {
    EVENTS.get()?.receiver.lock().ok()?.try_recv().ok()
}

fn enqueue(action: MenuAction) {
    if let Some(events) = EVENTS.get() {
        if events.sender.send(action).is_ok() {
            if let Some(proxy) = WAKE_PROXY.get() {
                let _ = proxy.send_event(crate::DesktopEvent::MenuWake);
            }
        }
    }
}

fn edit_canvas(action: MenuAction) {
    let mtm = MainThreadMarker::new().expect("AppKit action on main thread");
    let app = NSApplication::sharedApplication(mtm);
    // An unsupported native edit must not reach the editor behind the panel.
    let panel_active = unsafe { app.modalWindow().is_some() }
        || app
            .keyWindow()
            .is_some_and(|window| window.isKindOfClass(class!(NSPanel)));
    queue_canvas_edit(action, panel_active, enqueue);
}

declare_class!(
    pub(super) struct EditResponder;

    unsafe impl ClassType for EditResponder {
        #[inherits(NSObject)]
        type Super = NSResponder;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "OpenPencilEditResponder";
    }

    impl DeclaredClass for EditResponder { type Ivars = (); }

    unsafe impl EditResponder {
        #[method(cut:)]
        fn cut(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::Cut); }
        #[method(copy:)]
        fn copy(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::Copy); }
        #[method(paste:)]
        fn paste(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::Paste); }
        #[method(selectAll:)]
        fn select_all(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::SelectAll); }
        #[method(undo:)]
        fn undo(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::Undo); }
        #[method(redo:)]
        fn redo(&self, _sender: Option<&AnyObject>) { edit_canvas(MenuAction::Redo); }
    }
);

/// Own the inserted responder and restore the chain when its menu is dropped.
pub(super) struct CanvasEdits {
    view: Retained<NSView>,
    responder: Retained<EditResponder>,
    next: Option<Retained<NSResponder>>,
}

impl Drop for CanvasEdits {
    fn drop(&mut self) {
        // A locale rebuild may already have installed a new bridge. Never
        // detach that replacement or leave a dangling nextResponder pointer.
        unsafe {
            if self
                .view
                .nextResponder()
                .is_some_and(|next| std::ptr::eq::<NSResponder>(&*next, &**self.responder))
            {
                self.view.setNextResponder(self.next.as_deref());
            }
        }
    }
}

pub(super) fn bind(window: &winit::window::Window) -> Option<CanvasEdits> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let mtm = MainThreadMarker::new().expect("native menu installed on main thread");
    let RawWindowHandle::AppKit(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: the raw view belongs to this live winit window. We retain it and
    // the bridge, and restore its non-owning responder link on drop.
    unsafe {
        let view = Retained::retain(handle.ns_view.cast::<NSView>().as_ptr())?;
        let mut next = view.nextResponder();
        if let Some(old) = next
            .as_ref()
            .filter(|r| r.isKindOfClass(EditResponder::class()))
        {
            next = old.nextResponder();
        }
        let responder = mtm.alloc::<EditResponder>().set_ivars(());
        let responder: Retained<EditResponder> = msg_send_id![super(responder), init];
        responder.setNextResponder(next.as_deref());
        view.setNextResponder(Some(&responder));

        Some(CanvasEdits {
            view,
            responder,
            next,
        })
    }
}

fn queue_canvas_edit(
    action: MenuAction,
    native_panel_active: bool,
    enqueue: impl FnOnce(MenuAction),
) {
    if !native_panel_active {
        enqueue(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_panel_edits_do_not_leak_into_the_canvas_queue() {
        for action in [
            MenuAction::Cut,
            MenuAction::Copy,
            MenuAction::Paste,
            MenuAction::SelectAll,
            MenuAction::Undo,
            MenuAction::Redo,
        ] {
            let mut queued = Vec::new();
            queue_canvas_edit(action, true, |a| queued.push(a));
            assert!(queued.is_empty());
            queue_canvas_edit(action, false, |a| queued.push(a));
            assert_eq!(queued, [action]);
        }
    }
}
