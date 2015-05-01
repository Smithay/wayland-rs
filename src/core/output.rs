use std::cell::{Cell, RefCell};
use std::ffi::CStr;

use libc::{c_char, c_void};

use super::Registry;

use ffi::{abi, FFI, Bind};
use ffi::enums::{wl_output_mode, WL_OUTPUT_MODE_CURRENT, WL_OUTPUT_MODE_PREFERRED};
pub use ffi::enums::wl_output_subpixel as OutputSubpixel;
pub use ffi::enums::wl_output_transform as OutputTransform;

use ffi::interfaces::output::{wl_output, wl_output_listener, wl_output_add_listener,
                              wl_output_destroy};

/// An opaque unique identifier to an output, can be tested for equality.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct OutputId {
    p: usize
}

/// Representation of an output mode
#[derive(Copy, Clone)]
pub struct OutputMode {
    mode: wl_output_mode,
    /// width of the mode in hardware units
    pub width: i32,
    /// height of the mode in hardware units
    pub height: i32,
    /// vertical refresh rate in mHz
    pub refresh: i32
}

impl OutputMode {
    pub fn is_current(&self) -> bool {
        self.mode.intersects(WL_OUTPUT_MODE_CURRENT)
    }

    pub fn is_preferred(&self) -> bool {
        self.mode.intersects(WL_OUTPUT_MODE_PREFERRED)
    }
}

struct OutputData {
    position: Cell<(i32, i32)>,
    dimensions: Cell<(i32, i32)>,
    subpixel: Cell<OutputSubpixel>,
    manufacturer: RefCell<String>,
    model: RefCell<String>,
    transform: Cell<OutputTransform>,
    modes: RefCell<Vec<OutputMode>>,
    scale: Cell<i32>,
}

impl OutputData {
    fn new() -> OutputData {
        OutputData {
            position: Cell::new((0,0)),
            dimensions: Cell::new((0,0)),
            subpixel: Cell::new(OutputSubpixel::WL_OUTPUT_SUBPIXEL_UNKNOWN),
            manufacturer: RefCell::new(String::new()),
            model: RefCell::new(String::new()),
            transform: Cell::new(OutputTransform::WL_OUTPUT_TRANSFORM_NORMAL),
            modes: RefCell::new(Vec::new()),
            scale: Cell::new(1)
        }
    }

    fn handle_geometry(&self, position: (i32, i32),
                              dimensions: (i32, i32),
                              subpixel: OutputSubpixel,
                              manufacturer: String,
                              model: String,
                              transform: OutputTransform) {
        self.position.set(position);
        self.dimensions.set(dimensions);
        self.subpixel.set(subpixel);
        *self.manufacturer.borrow_mut() = manufacturer;
        *self.model.borrow_mut() = model;
        self.transform.set(transform);
    }

    fn handle_mode(&self, output_mode: wl_output_mode, width: i32, height: i32, refresh: i32) {
        let mut guard = self.modes.borrow_mut();
        for mode in &mut *guard {
            if mode.width == width && mode.height == height && mode.refresh == refresh {
                mode.mode = output_mode;
                return;
            }
        }
        guard.push(OutputMode {
            mode: output_mode,
            width: width,
            height: height,
            refresh: refresh
        });
    }

    fn handle_scale(&self, scale: i32) {
        self.scale.set(scale);
    }
}

struct OutputListener {
    geometry_handler: Box<Fn(i32, i32, i32, i32, OutputSubpixel, &[u8], &[u8], OutputTransform, &OutputData)>,
    mode_handler: Box<Fn(wl_output_mode, i32, i32, i32, &OutputData)>,
    done_handler: Box<Fn(&OutputData)>,
    scale_handler: Box<Fn(i32, &OutputData)>,
    data: OutputData
}

impl OutputListener {
    fn default_handlers(data: OutputData) -> OutputListener {
        OutputListener {
            geometry_handler: Box::new(|x, y, w, h, sub, manu, model, s, data| {
                data.handle_geometry(
                    (x, y),
                    (w, h),
                    sub,
                    String::from_utf8_lossy(manu).into_owned(),
                    String::from_utf8_lossy(model).into_owned(),
                    s
                )
            }),
            mode_handler: Box::new(|m, w, h, r, data| {
                data.handle_mode(m, w, h, r)
            }),
            done_handler: Box::new(|_| {}),
            scale_handler: Box::new(|s, data| {
                data.handle_scale(s)
            }),
            data: data
        }
    }
}

/// A physical output
pub struct Output {
    _registry: Registry,
    ptr: *mut wl_output,
    listener: Box<OutputListener>
}

impl Output {
    /// The location of the top-left corner of this output in the
    /// compositor space.
    pub fn position(&self) -> (i32, i32) {
        self.listener.data.position.get()
    }

    /// The dimensions (width, height) of this output, in milimeters
    pub fn dimensions(&self) -> (i32, i32) {
        self.listener.data.dimensions.get()
    }

    /// The subpixel orientation of this output
    pub fn subpixel(&self) -> OutputSubpixel {
        self.listener.data.subpixel.get()
    }

    /// The current transform of this output
    pub fn transform(&self) -> OutputTransform {
        self.listener.data.transform.get()
    }

    /// The manufacturer of this output
    pub fn manufacturer(&self) -> String {
        self.listener.data.manufacturer.borrow().clone()
    }

    /// The model of this output
    pub fn model(&self) -> String {
        self.listener.data.model.borrow().clone()
    }

    /// The current scaling factor of this output
    pub fn scale(&self) -> i32 {
        self.listener.data.scale.get()
    }

    /// The modes of this output
    pub fn modes(&self) -> Vec<OutputMode> {
        self.listener.data.modes.borrow().clone()
    }
}

impl Bind<Registry> for Output {
    fn interface() -> &'static abi::wl_interface {
        abi::WAYLAND_CLIENT_HANDLE.wl_output_interface
    }

    unsafe fn wrap(ptr: *mut wl_output, registry: Registry) -> Output {
        let listener_data = OutputListener::default_handlers(OutputData::new());
        let o = Output {
            _registry: registry,
            ptr: ptr,
            listener: Box::new(listener_data)
        };
        wl_output_add_listener(
            o.ptr,
            &OUTPUT_LISTENER as *const _,
            &*o.listener as *const _ as *mut _
        );
        o
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        unsafe { wl_output_destroy(self.ptr) };
    }
}

impl FFI for Output {
    type Ptr = wl_output;

    fn ptr(&self) -> *const wl_output {
        self.ptr as *const wl_output
    }

    unsafe fn ptr_mut(&self) -> *mut wl_output {
        self.ptr
    }
}

//
// C-wrappers for the callback closures, to send to wayland
//
extern "C" fn output_geometry_handler(data: *mut c_void,
                                      _output: *mut wl_output,
                                      x: i32,
                                      y: i32,
                                      width: i32,
                                      height: i32,
                                      subpixel: OutputSubpixel,
                                      manufacturer: *const c_char,
                                      model: *const c_char,
                                      transform: OutputTransform
                                     ) {
    let listener = unsafe { &*(data as *const OutputListener) };
    let manu_str = unsafe { CStr::from_ptr(manufacturer) };
    let model_str = unsafe { CStr::from_ptr(model) };
    (*listener.geometry_handler)(
        x, y, width, height, subpixel,
        manu_str.to_bytes(),model_str.to_bytes(), transform,
        &listener.data
    );
}

extern "C" fn output_mode_handler(data: *mut c_void,
                                  _output: *mut wl_output,
                                  flags: wl_output_mode,
                                  width: i32,
                                  height: i32,
                                  refresh: i32
                                 ) {
    let listener = unsafe { &*(data as *const OutputListener) };
    (*listener.mode_handler)(
        flags, width, height, refresh,
        &listener.data
    );
}

extern "C" fn output_done_handler(data: *mut c_void,
                                  _output: *mut wl_output
                                 ) {
    let listener = unsafe { &*(data as *const OutputListener) };
    (*listener.done_handler)(&listener.data);
}

extern "C" fn output_scale_handler(data: *mut c_void,
                                   _output: *mut wl_output,
                                   scale: i32
                                  ) {
    let listener = unsafe { &*(data as *const OutputListener) };
    (*listener.scale_handler)(scale, &listener.data);
}

static OUTPUT_LISTENER: wl_output_listener = wl_output_listener {
    geometry: output_geometry_handler,
    mode: output_mode_handler,
    done: output_done_handler,
    scale: output_scale_handler
};