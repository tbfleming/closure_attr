use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

struct WithCallback {
    dropped: Rc<Cell<bool>>,
    called: Cell<bool>,
    callback: RefCell<Option<Box<dyn FnMut()>>>,
}

impl Drop for WithCallback {
    fn drop(&mut self) {
        self.dropped.replace(true);
    }
}

fn run_static_callback<R, F: FnOnce() -> R + 'static>(f: F) -> R {
    f()
}

#[test]
#[closure_attr::with_closure]
fn prevent_rc_loop() {
    let dropped = Rc::new(Cell::new(false));

    let x = Rc::new(WithCallback {
        dropped: dropped.clone(),
        called: Cell::new(false),
        callback: RefCell::new(None),
    });

    let callback = #[closure(weak x)]
    move || {
        x.upgrade().unwrap().called.set(true);
    };
    (*x.callback.borrow_mut()) = Some(Box::new(callback));

    assert!(!x.called.get());
    (*x.callback.borrow_mut()).as_mut().unwrap()();
    assert!(x.called.get());

    assert!(!dropped.get());
    drop(x);
    assert!(dropped.get());
}

#[test]
#[closure_attr::with_closure]
#[allow(clippy::needless_return)]
fn return_in_body() {
    let i = Rc::new(42);
    let callback = #[closure(clone i)]
    move || {
        return *i;
    };
    assert_eq!(callback(), 42);
}

#[test]
#[closure_attr::with_closure]
#[allow(clippy::needless_return)]
fn return_in_panic_body() {
    let i = Arc::new(42);
    let callback = #[closure(panic i)]
    move || {
        return *i;
    };
    assert_eq!(callback(), 42);
}

#[test]
#[closure_attr::with_closure]
fn live_fail() {
    let i = Arc::new(42);
    let callback = #[closure(fail(7) i)]
    move || *i;
    assert_eq!(callback(), 42);
}

#[test]
#[closure_attr::with_closure]
fn dead_fail() {
    let i = Arc::new(42);
    let callback = #[closure(fail(7) i)]
    move || *i;
    drop(i);
    assert_eq!(callback(), 7);
}

#[test]
#[closure_attr::with_closure]
#[allow(clippy::needless_return)]
fn live_panic() {
    let i = Arc::new(42);
    let callback = #[closure(panic i)]
    move || {
        return *i;
    };
    assert_eq!(callback(), 42);
}

#[test]
#[should_panic(expected = "Closure failed to upgrade weak pointer")]
#[closure_attr::with_closure]
#[allow(clippy::needless_return)]
fn dead_panic() {
    let i = Arc::new(42);
    let callback = #[closure(panic i)]
    move || {
        return *i;
    };
    drop(i);
    callback();
}

#[test]
#[closure_attr::with_closure]
#[allow(clippy::needless_return)]
fn embedded_closure() {
    let i = Rc::new(42);
    let callback = #[closure(clone i)]
    move || {
        let inner = #[closure(clone i)]
        move || {
            return *i;
        };
        (inner, i)
    };
    let (inner, i2) = run_static_callback(callback);
    assert_eq!(*i2, 42);
    assert_eq!(run_static_callback(inner), 42);
    assert_eq!(*i, 42);
}

#[test]
#[allow(clippy::no_effect)]
fn capture_whole() {
    // Test by https://github.com/steffahn
    // Compile will fail if only p.0 is captured
    fn send<T: Send>(_: T) {}

    #[derive(Clone)]
    struct SendPointer(*const ());
    unsafe impl Send for SendPointer {}

    #[closure_attr::with_closure]
    fn f() {
        let p = SendPointer(std::ptr::null());
        send(
            #[closure(clone p)]
            move || {
                p.0;
            },
        );
    }

    let _ = f;
}
