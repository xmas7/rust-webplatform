#![feature(plugin)]
#![feature(core)]
#![plugin(concat_bytes)]

extern crate libc;

#[macro_use]
mod webplatform;

use webplatform::{HtmlNode, alert};

fn main() {
	webplatform::init();

    let mut body = HtmlNode::query("body").unwrap();

    let hr = HtmlNode::create("hr").unwrap();
    body.append(&hr);

    body.html_prepend("<h1>HELLO FROM RUST</h1>");
    body.html_append("<button>CLICK ME</button>");
    let mut button = HtmlNode::query("button").unwrap();
    
    button.on("click", || {
    	body.prop_set_str("bgColor", "blue");
    });

    println!("This should be blue: {:?}", body.prop_get_str("bgColor"));
    println!("Height?: {:?}", body.prop_get_i32("clientWidth"));
}
