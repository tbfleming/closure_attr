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

```text
use_callback({
    let s = s.clone();
    let i = i.clone();
    move || {
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
| `rcweak <ident>` | See below |
| `arcweak <ident>` | See below |

## `rcweak` and `arcweak`

`rcweak` and `arcweak` use weak pointers to help break up reference cycles.
They downgrade an `Rc` or `Arc` pointer and capture it. The transformed
closure upgrades the reference when it is called. If any upgrade fails, it skips
executing the body and returns `Default::default()`.

```rust
use std::{rc::Rc, sync::Arc};

#[closure_attr::with_closure]
fn example() {
    let r = Rc::new(3);
    let a = Arc::new(4);

    let closure = #[closure(rcweak r, arcweak a)]
    move || *r * *a;

    assert_eq!(closure(), 12);
}

example();
```

This Expands to:

```text
let closure = {
    let r = ::std::rc::Rc::downgrade(&r);
    let a = ::std::sync::Arc::downgrade(&a);
    move || {
        (|| {
            let r = r.upgrade()?;
            let a = a.upgrade()?;
            Some((|| *r * *a)())
        })()
        .unwrap_or_default()
    }
};
```

## License

This work is dual-licensed under MIR and Apache 2.0.
You can choose between one of them if you use this work.

`SPDX-License-Identifier: MIT OR Apache-2.0`
