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
| `weak <ident>` | See below |

## `weak`

`weak` uses weak pointers to help break up reference cycles. It downgrades
an `Rc` or `Arc` pointer (or anything which implements [Downgrade] and [Upgrade])
and captures it. The transformed closure upgrades the pointer when it is called.
If any upgrade fails, it skips executing the body and returns `Default::default()`.

```rust
use std::{rc::Rc, sync::Arc};

#[closure_attr::with_closure]
fn example() {
    let r = Rc::new(3);
    let a = Arc::new(4);

    let closure = #[closure(weak r, weak a)]
    move || *r * *a;

    assert_eq!(closure(), 12);
}

example();
```

This Expands to:

```ignore
let closure = {
    let r = ::closure_attr::Downgrade::downgrade(&r);
    let a = ::closure_attr::Downgrade::downgrade(&a);
    move || {
        (|| {
            let r = ::closure_attr::Upgrade::upgrade(&r)?;
            let a = ::closure_attr::Upgrade::upgrade(&a)?;
            Some((|| *r * *a)())
        })()
        .unwrap_or_default()
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
