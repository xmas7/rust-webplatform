#![allow(dead_code)]

use libc;
use std::ffi::CString;
use std::mem;
use std::mem::forget;

trait Interop {
    fn as_int(self, arena:&mut Vec<CString>) -> libc::c_int;
}

impl Interop for i32 {
    fn as_int(self, arena:&mut Vec<CString>) -> libc::c_int {
        return self;
    }
}

impl<'a> Interop for &'a str {
    fn as_int(self, arena:&mut Vec<CString>) -> libc::c_int {
        let c = CString::new(self).unwrap();
        let ret = c.as_ptr() as libc::c_int;
        arena.push(c);
        return ret;
    }
}

impl<'a> Interop for *const libc::c_void {
    fn as_int(self, arena:&mut Vec<CString>) -> libc::c_int {
        return self as libc::c_int;
    }
}

macro_rules! js {
    ( ($( $x:expr ),*) $y:expr ) => {
        unsafe {
            use webplatform;
            let mut arena:Vec<CString> = Vec::new();
            webplatform::emscripten_asm_const_int(concat_bytes!($y, b"\0").as_ptr() as *const libc::c_char, $(Interop::as_int($x, &mut arena)),*)
        }
    };
    ( $y:expr ) => {
        unsafe {
            use webplatform;
            webplatform::emscripten_asm_const_int(concat_bytes!($y, b"\0").as_ptr() as *const libc::c_char)
        }
    };
}

extern {
    pub fn emscripten_asm_const(s: *const libc::c_char);
    pub fn emscripten_asm_const_int(s: *const libc::c_char, ...) -> libc::c_int;
}

pub struct HtmlNode {
    id: libc::c_int,
}

extern fn rust_caller<F: Fn()>(a: *const libc::c_void) {
    let v:&F = unsafe { mem::transmute(a) };
    v();
}

impl HtmlNode {
    pub fn create(s: &str) -> Option<HtmlNode> {
        let id = js! { (s) br#"
            var value = document.createElement(UTF8ToString($0));
            if (!value) {
                return -1;
            }
            return WEBPLATFORM.rs_refs.push(value) - 1;
        "#};

        if id < 0 {
            None
        } else {
            Some(HtmlNode {
                id: id,
            })
        }
    }

    pub fn query(s: &str) -> Option<HtmlNode> {
        let id = js! { (s) br#"
            var value = document.querySelector(UTF8ToString($0));
            if (!value) {
                return -1;
            }
            return WEBPLATFORM.rs_refs.push(value) - 1;
        "#};

        if id < 0 {
            None
        } else {
            Some(HtmlNode {
                id: id,
            })
        }
    }

    pub fn html_set(&self, s: &str) {
        js! { (self.id, s) br#"
            WEBPLATFORM.rs_refs[$0].innerHTML = UTF8ToString($1);
        "#};
    }
    
    pub fn prop_set_i32(&self, s: &str, v: i32) {
        js! { (self.id, s, v) concat_bytes!(br#"
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = $2;
        "#)};
    }
    
    pub fn prop_set_str(&self, s: &str, v: &str) {
        println!("s {:?}", s);
        println!("v {:?}", v);
        js! { (self.id, s, v) concat_bytes!(br#"
            console.log($1, $2);
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = UTF8ToString($2);
        "#)};
    }
    
    // pub fn prop_get_str(&self, s: &str) -> String {
    //     let a = js! { (self.id, s) concat_bytes!(br#"
    //         return WEBPLATFORM.rs_refs[$0][UTF8ToString($1)]
    //     "#)};
    // }

    pub fn append(&self, s: &HtmlNode) {
        js! { (self.id, s.id) br#"
            WEBPLATFORM.rs_refs[$0].appendChild(WEBPLATFORM.rs_refs[$1]);
        "#};
    }

    pub fn html_append(&self, s: &str) {
        js! { (self.id, s) br#"
            WEBPLATFORM.rs_refs[$0].insertAdjacentHTML('beforeEnd', UTF8ToString($1));
        "#};
    }

    pub fn html_prepend(&self, s: &str) {
        js! { (self.id, s) br#"
            WEBPLATFORM.rs_refs[$0].insertAdjacentHTML('afterBegin', UTF8ToString($1));
        "#};
    }

    pub fn on<F: Fn()>(&mut self, s: &str, f: F) {
        unsafe {
            let a = &f as *const _;
            forget(f);
            js! { (self.id, s, a as *const libc::c_void, rust_caller::<F> as *const libc::c_void) br#"
                WEBPLATFORM.rs_refs[$0].addEventListener(UTF8ToString($1), function () {
                    Runtime.dynCall('vi', $3, [$2]);
                }, false);
            "#};
        }
    }
}

impl Drop for HtmlNode {
    fn drop(&mut self) {
        js! { (self.id) br#"
            delete WEBPLATFORM.rs_refs[$0];
        "#};
    }
}

pub fn alert(s: &str) {
    js! { (s) br#"
        alert(UTF8ToString($0));
    "#};
}

pub fn init() {
    js! { br#"
        this.WEBPLATFORM = {
            rs_refs: [],
        };
    "#};
}
