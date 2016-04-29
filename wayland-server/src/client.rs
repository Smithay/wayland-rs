use wayland_sys::server::wl_client;

pub struct Client {
    ptr: *mut wl_client
}

impl Client {
    pub fn ptr(&self) -> *mut wl_client {
        self.ptr
    }

    pub fn id(&self) -> ClientId {
        wrap_client(self.ptr)
    }
}

#[derive(Copy,Clone,PartialEq,Eq,Debug,Hash)]
pub struct ClientId { id: usize }

pub fn wrap_client(ptr: *mut wl_client) -> ClientId {
    ClientId { id: ptr as usize}
}
