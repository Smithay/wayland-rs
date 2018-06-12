use Interface;

use imp::GlobalInner;

/// A handle to a global object
///
/// This is given to you when you register a global to the event loop.
///
/// This handle allows you do destroy the global when needed.
///
/// If you know you will never destroy this global, you can let this
/// handle go out of scope.
pub struct Global<I: Interface> {
    inner: GlobalInner<I>,
}

impl<I: Interface> Global<I> {
    pub(crate) fn create(inner: GlobalInner<I>) -> Global<I> {
        Global { inner }
    }

    /// Destroy the associated global object.
    pub fn destroy(self) {
        self.inner.destroy()
    }
}
