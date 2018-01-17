extern crate tera;
#[macro_use]
extern crate serde_derive;

use tera::{Tera, Context, Result};

mod common;
use common::{Product, Review};


fn render_tpl(tpl_name: &str) -> Result<String> {
    let tera = Tera::new("tests/render-success/**/*").unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);

    tera.render(tpl_name, &context)
}


#[test]
fn test_render_math_length() {
    let result = render_tpl("math_length.html");

    assert_eq!(result.unwrap(), "1");
}
