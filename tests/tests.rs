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
        x.called.set(true);
    };
    (*x.callback.borrow_mut()) = Some(Box::new(callback));

    assert_eq!(x.called.get(), false);
    (*x.callback.borrow_mut()).as_mut().unwrap()();
    assert_eq!(x.called.get(), true);

    assert_eq!(dropped.get(), false);
    drop(x);
    assert_eq!(dropped.get(), true);
}

#[test]
#[closure_attr::with_closure]
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
fn return_in_weak_body() {
    let i = Arc::new(42);
    let callback = #[closure(weak i)]
    move || {
        return *i;
    };
    assert_eq!(callback(), 42);
}

#[test]
#[closure_attr::with_closure]
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
