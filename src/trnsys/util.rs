use std::os::raw::c_int;

pub fn c_bool(i: c_int) -> bool {
    i != 0
}

