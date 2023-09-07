#![doc = include_str!("../README.md")]

/// See the [crate-level documentation](index.html).
pub use closure_attr_derive::with_closure;

/// Types which can be downgraded to their weak form,
/// such as [`std::rc::Rc`] and [`std::sync::Arc`].
pub trait Downgrade: Sized {
    /// The weak form of Self.
    type Target: Upgrade<Target = Self>;

    /// Downgrade Self to its weak form.
    fn downgrade(this: &Self) -> Self::Target;
}

/// Types which can be upgraded from their weak form,
/// such as [`std::rc::Weak`] and [`std::sync::Weak`].
pub trait Upgrade {
    /// The strong form of Self.
    type Target: Downgrade;

    /// Upgrade Self to its strong form.
    fn upgrade(&self) -> Option<Self::Target>;
}

impl<T> Downgrade for std::rc::Rc<T> {
    type Target = std::rc::Weak<T>;
    fn downgrade(this: &Self) -> Self::Target {
        std::rc::Rc::downgrade(this)
    }
}

impl<T> Upgrade for std::rc::Weak<T> {
    type Target = std::rc::Rc<T>;
    fn upgrade(&self) -> Option<Self::Target> {
        self.upgrade()
    }
}

impl<T> Downgrade for std::sync::Arc<T> {
    type Target = std::sync::Weak<T>;
    fn downgrade(this: &Self) -> Self::Target {
        std::sync::Arc::downgrade(this)
    }
}

impl<T> Upgrade for std::sync::Weak<T> {
    type Target = std::sync::Arc<T>;
    fn upgrade(&self) -> Option<Self::Target> {
        self.upgrade()
    }
}
