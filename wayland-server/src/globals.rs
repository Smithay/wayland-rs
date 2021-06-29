use crate::{Interface, Resource};

use crate::imp::GlobalInner;

/// A handle to a global object
///
/// This is given to you when you register a global to the event loop.
///
/// This handle allows you do destroy the global when needed.
///
/// If you know you will never destroy this global, you can let this
/// handle go out of scope.
pub struct Global<I: Interface + AsRef<Resource<I>> + From<Resource<I>>> {
    inner: GlobalInner<I>,
}

impl<I: Interface + AsRef<Resource<I>> + From<Resource<I>>> std::fmt::Debug for Global<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Global { ... }")
    }
}

impl<I: Interface + AsRef<Resource<I>> + From<Resource<I>>> Global<I> {
    pub(crate) fn create(inner: GlobalInner<I>) -> Global<I> {
        Global { inner }
    }

    /// Destroys the associated global object.
    pub fn destroy(self) {
        self.inner.destroy()
    }
}
