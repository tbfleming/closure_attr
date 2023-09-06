use std::{
    cell::{Cell, RefCell},
    rc::Rc,
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

#[test]
#[closure_attr::with_closure]
fn prevent_rc_loop() {
    let dropped = Rc::new(Cell::new(false));

    let x = Rc::new(WithCallback {
        dropped: dropped.clone(),
        called: Cell::new(false),
        callback: RefCell::new(None),
    });

    let callback = #[closure(rcweak x)]
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
