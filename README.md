# closure_attr

This crate provides an attribute to simplify closure captures.

## Example

```rust
use std::{rc::Rc, cell::Cell, cell::RefCell};

// Expects a 'static callback
fn use_callback<F: FnMut() + 'static>(mut callback: F) {
    callback();
}

#[closure_attr::with_closure] // Enable use of #[closure(...)]
fn example() {
    let s = Rc::new(RefCell::new(String::new()));
    let i = Rc::new(Cell::new(0));

    // The callback captures clones of s and i
    use_callback(
        #[closure(clone s, clone i)]
        move || {
            s.replace(format!("Hello, world! {}", i.get()));
            i.set(i.get() + 1);
        },
    );

    assert_eq!(s.borrow_mut().clone(), "Hello, world! 0");
    assert_eq!(i.get(), 1);
}

example();
```

It expands to:

```ignore
use_callback({
    let s = s.clone(); // Clone requested by attribute
    let i = i.clone(); // Clone requested by attribute
    move || {
        {... code to force whole captures ...}
        s.replace(format!("Hello, world! {}", i.get()));
        i.set(i.get() + 1);
    }
});
```

## Capture types

| Syntax | Description |
| --- | --- |
| `clone <ident>` | Clone the variable |
| `clone mut <ident>` | Clone the variable and make it mutable |
| `ref <ident>` | Take a reference to the variable |
| `ref mut <ident>` | Take a mutable reference to the variable |
| `move <ident>` | Move the variable into the closure |
| `move mut <ident>` | Move the variable into the closure and make it mutable |
| `weak <ident>` | Downgrade an `Rc`, `Arc`, or anything else which implements [Downgrade]. Captures the downgraded pointer. This helps break up reference loops. |
| `fail(<expr>) <ident>` | Like `weak`, but upgrades the weak pointer before executing the closure body. If the upgrade fails, it skips executing the body and returns the expression. |
| `panic <ident>` | Like `weak`, but upgrades the weak pointer before executing the closure body. If the upgrade fails, it panics with message "Closure failed to upgrade weak pointer". |

## `weak`, `fail`, and `panic` transforms

```rust
use std::{rc::Rc, cell::Cell, cell::RefCell};

#[closure_attr::with_closure]
fn weak_examples() {
    let i = Rc::new(42);

    let weak = #[closure(weak i)]
    move || *i.upgrade().unwrap() + 1; // manual upgrade

    let fail = #[closure(fail(7) i)]
    move || *i + 2;

    let panic = #[closure(panic i)]
    move || *i + 3;

    assert_eq!(weak(), 43);
    assert_eq!(fail(), 44);
    assert_eq!(panic(), 45);
}

weak_examples();
```

The closures expand to:

```ignore
let weak = {
    let i = ::closure_attr::Downgrade::downgrade(&i);
    move || *i.upgrade().unwrap() + 1 // manual upgrade
};

let fail = {
    let i = ::closure_attr::Downgrade::downgrade(&i);
    move || {
        let Some(i) = ::closure_attr::Upgrade::upgrade(&i) else {
            return 7;
        };
        *i + 2
    }
};

let panic = {
    let i = ::closure_attr::Downgrade::downgrade(&i);
    move || {
        let Some(i) = ::closure_attr::Upgrade::upgrade(&i) else {
            ::std::panic!("Closure failed to upgrade weak pointer");
        };
        *i + 3
    }
};
```

## Whole captures

The `capture` attribute captures whole variables. For example, this code without the attribute produces an error:

```ignore
fn send<T: Send>(_: T) {}

struct SendPointer(*const ());
unsafe impl Send for SendPointer {}

fn f() {
    let p = SendPointer(std::ptr::null());
    send(
        move || {
            p.0;
        },
    );
}
```

```text
error[E0277]: `*const ()` cannot be sent between threads safely
```

A workaround:

```ignore
#[closure_attr::with_closure]
fn f() {
    let p = SendPointer(std::ptr::null());
    send(
        #[closure(move p)]
        move || {
            p.0;
        },
    );
}
```

This is equivalent to inserting `let _ = &p;` into the body of the closure.

## License

This work is dual-licensed under MIT and Apache 2.0.
You can choose between one of them if you use this work.

`SPDX-License-Identifier: MIT OR Apache-2.0`
