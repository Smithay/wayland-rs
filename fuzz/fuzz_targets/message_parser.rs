#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate wayland_commons;

use std::{mem, slice};
use std::os::unix::io::RawFd;
use wayland_commons::wire::{Message, ArgumentType};

unsafe fn convert_slice<T: Sized>(data: &[u8]) -> &[T] {
    let n = mem::size_of::<T>();
    slice::from_raw_parts(
        data.as_ptr() as *const T,
        data.len()/n,
    )
}

fn get_arg_types(data: &[u8]) -> [ArgumentType; 16] {
    use ArgumentType::*;

    let mut res = [Int; 16];
    assert_eq!(data.len(), 16);
    for i in 0..16 {
        res[i] = match data[i] & 0b111 {
            0 => Int,
            1 => Uint,
            2 => Fixed,
            3 => Str,
            4 => Object,
            5 => NewId,
            6 => Array,
            7 => Fd,
            _ => unreachable!(),
        }
    }
    res
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 32 { return; }
    // 4 `RawFd`s
    let fds: &[RawFd] = unsafe { convert_slice(&data[..16]) };
    // 16 `ArgumentType`s
    let args = get_arg_types(&data[16..32]);
    let data: &[u32] = unsafe { convert_slice(&data[32..]) };
    let _res = Message::from_raw(data, &args, fds);
});
