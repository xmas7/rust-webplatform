#[macro_use] extern crate webplatform;
extern crate libc;

use std::borrow::ToOwned;

fn main() {
    let document = webplatform::init();
    {
        let body = document.element_query("body").unwrap();

        let hr = document.element_create("hr").unwrap();
        body.append(&hr);

        body.html_prepend("<h1>HELLO FROM RUST</h1>");
        body.html_append("<button>CLICK ME</button>");

        let mut button = document.element_query("button").unwrap();

        let bodyref = body.root_ref();
        let bodyref2 = body.root_ref();
        button.on("click", move |_| {
            bodyref2.prop_set_str("bgColor", "blue");
            println!("This should be string 'blue': {:?}", bodyref2.prop_get_str("bgColor"));
        });

        println!("This should be empty string: {:?}", bodyref.prop_get_str("bgColor"));
        println!("Width?: {:?}", bodyref.prop_get_i32("clientWidth"));

        webplatform::spin();
    }

    println!("NO CALLING ME.");
}
