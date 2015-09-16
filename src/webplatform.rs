#![allow(dead_code)]

use libc;
use std::ffi::{CString, CStr};
use std::{mem, fmt};
use std::str;
use std::borrow::ToOwned;
use std::ops::Deref;
use std::cell::RefCell;

trait Interop {
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int;
}

impl Interop for i32 {
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int {
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
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int {
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
    pub fn emscripten_pause_main_loop();
    pub fn emscripten_set_main_loop(m: extern fn(), fps: libc::c_int, infinite: libc::c_int);
}

pub struct HtmlNode<'a> {
    id: libc::c_int,
    refs: RefCell<Vec<Box<FnMut() + 'a>>>,
}

impl<'a> fmt::Debug for HtmlNode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HtmlNode({:?})", self.id)
    }
}

#[unsafe_destructor]
impl<'a> Drop for HtmlNode<'a> {
    fn drop(&mut self) {
        println!("dropping HTML NODE {:?}", self.id);
    }
}

pub struct JSRef<'a> {
    ptr: *const HtmlNode<'a>,
}

impl<'a> fmt::Debug for JSRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JSRef(HtmlNode({:?}))", self.id)
    }
}

use std::clone::Clone;

impl<'a> Clone for JSRef<'a> {
    fn clone(&self) -> JSRef<'a> {
        JSRef {
            ptr: self.ptr,
        }
    }
}

impl<'a> HtmlNode<'a> {
    pub fn root_ref<'b>(&'b self) -> JSRef<'a> {
        JSRef {
            ptr: &*self,
        }
    }
}

impl<'a> Deref for JSRef<'a> {
    type Target = HtmlNode<'a>;

    fn deref(&self) -> &HtmlNode<'a> {
        unsafe {
            &*self.ptr
        }
    }
}

extern fn rust_caller<F: FnMut()>(a: *const libc::c_void) {
    let v:&mut F = unsafe { mem::transmute(a) };
    v();
}

impl<'a> HtmlNode<'a> {
    pub fn html_set(&self, s: &str) {
        js! { (self.id, s) br#"
            WEBPLATFORM.rs_refs[$0].innerHTML = UTF8ToString($1);
        "#};
    }
    
    pub fn prop_set_i32(&self, s: &str, v: i32) {
        js! { (self.id, s, v) br#"
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = $2;
        "#};
    }
    
    pub fn prop_set_str(&self, s: &str, v: &str) {
        js! { (self.id, s, v) br#"
            console.log($0)
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = UTF8ToString($2);
        "#};
    }
    
    pub fn prop_get_i32(&self, s: &str) -> i32 {
        return js! { (self.id, s) concat_bytes!(br#"
            return WEBPLATFORM.rs_refs[$0][UTF8ToString($1)]
        "#)};
    }
    
    pub fn prop_get_str(&self, s: &str) -> String {
        let a = js! { (self.id, s) concat_bytes!(br#"
            return allocate(intArrayFromString(WEBPLATFORM.rs_refs[$0][UTF8ToString($1)]), 'i8', ALLOC_STACK);
        "#)};
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

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

    pub fn on<F: FnMut() + 'a>(&self, s: &str, f: F) {
        unsafe {
            let b = Box::new(f);
            let a = &*b as *const _;
            js! { (self.id, s, a as *const libc::c_void, rust_caller::<F> as *const libc::c_void) br#"
                WEBPLATFORM.rs_refs[$0].addEventListener(UTF8ToString($1), function () {
                    Runtime.dynCall('vi', $3, [$2]);
                }, false);
            "#};
            self.refs.borrow_mut().push(b);
        }
    }
}

pub fn alert(s: &str) {
    js! { (s) br#"
        alert(UTF8ToString($0));
    "#};
}

pub struct Document {
    pub refs: Vec<Box<FnMut()>>,
}

impl Document {
    pub fn element_create(&self, s: &str) -> Option<HtmlNode> {
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
                refs: RefCell::new(Vec::new()),
            })
        }
    }

    pub fn element_query(&self, s: &str) -> Option<HtmlNode> {
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
                refs: RefCell::new(Vec::new()),
            })
        }
    }
}

pub fn init() -> Document {
    js! { br#"
        this.WEBPLATFORM = {
            rs_refs: [],
        };
    "#};
    Document {
        refs: Vec::new()
    }
}

extern fn leavemebe() {
    unsafe {
        emscripten_pause_main_loop();
    }
}

pub fn spin() {
    unsafe {
        emscripten_set_main_loop(leavemebe, 0, 1);
        
    }
}
