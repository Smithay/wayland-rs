#[macro_escape]
macro_rules! external_library(
    (__struct, $structname: ident, $($name: ident: $proto: ty),+) => (
    pub struct $structname {
        $(pub $name: $proto),+
    }
    );
    (__impl, $structname: ident, $($name: ident),+) => (
    impl $structname {
        pub fn open(name: &str) -> Option<$structname> {
            let cname = match ::std::ffi::CString::new(name) {
                Ok(cs) => cs,
                Err(_) => return None
            };
            unsafe {
                let dl = $crate::ffi::dlopen(cname.as_bytes_with_nul().as_ptr() as *const _, 1);
                if dl.is_null() {
                    println!("dlopen failed");
                    return None;
                }
                $crate::ffi::dlerror();
                let s = $structname {
                    $($name: {
                        let s_name = concat!(stringify!($name), "\0");
                        let symbol = $crate::ffi::dlsym(dl, s_name.as_ptr() as *const _);
                        if !$crate::ffi::dlerror().is_null() {
                            println!("fetching symbol `{}` failed: {:p}", s_name, symbol);
                            return None
                        }
                        ::std::mem::transmute(symbol)
                    }
                    ),+
                };
                Some(s)
            }
        }
    }
    );
    ($structname: ident, $($name: ident: $proto: ty),+) => (
        external_library!(__struct, $structname, $($name: $proto),+);
        external_library!(__impl, $structname, $($name),+);
        unsafe impl Sync for $structname {}
    )
);
