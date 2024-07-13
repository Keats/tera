//! These tests demonstrate how to define structs that can be used as a `RenderContext`
//! one advantage to custom structs here is that we can use borrowed data instead of converting
//! the entire input into a `Value` which can be expensive.

use std::borrow::Cow;

use elsa::FrozenMap;
use serde_derive::Serialize;
use serde_json::Value;

use crate::{Context, RenderContext, Tera};

struct CachingContext<C: RenderContext> {
    inner: C,
    value_cache: FrozenMap<String, Box<Option<Value>>>,
}

impl<C: RenderContext> CachingContext<C> {
    fn new(inner: C) -> Self {
        Self { inner, value_cache: FrozenMap::default() }
    }
}

impl<C: RenderContext> RenderContext for CachingContext<C> {
    fn find_value<'k>(&self, key: &'k str) -> Option<Cow<Value>> {
        if let Some(cached) = self.value_cache.get(key) {
            cached.as_ref().map(Cow::Borrowed)
        } else {
            self.value_cache.insert(
                key.to_string(),
                Box::new(self.inner.find_value(key).map(|v| v.into_owned())),
            );
            self.value_cache.get(key).map(|v| v.as_ref().map(Cow::Borrowed)).flatten()
        }
    }

    fn deep_copy_as_context(&self) -> Context {
        self.inner.deep_copy_as_context()
    }
}

#[derive(Serialize)]
struct Company<'c> {
    name: &'c str,
    address: &'c str,
}

#[derive(Serialize)]
struct Customer<'c> {
    name: &'c str,
    address: &'c str,
}

#[derive(Serialize)]
struct Product<'p> {
    name: &'p str,
    sku: &'p str,
    company: &'p Company<'p>,
    price_in_cents: u32,
}

struct Order<'o> {
    customer: &'o Customer<'o>,
    product: &'o Product<'o>,
}

impl<'a> RenderContext for Order<'a> {
    fn find_value<'k>(&self, key: &'k str) -> Option<Cow<Value>> {
        match key {
            "customer.name" => Some(Cow::Owned(Value::String(self.customer.name.to_string()))),
            "customer.address" => {
                Some(Cow::Owned(Value::String(self.customer.address.to_string())))
            }
            "product.name" => Some(Cow::Owned(Value::String(self.product.name.to_string()))),
            "product.sku" => Some(Cow::Owned(Value::String(self.product.sku.to_string()))),
            "product.company.name" => {
                Some(Cow::Owned(Value::String(self.product.company.name.to_string())))
            }
            "product.price_in_cents" => Some(Cow::Owned(self.product.price_in_cents.into())),
            _ => None,
        }
    }

    fn deep_copy_as_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.insert("customer", self.customer);
        ctx.insert("product", self.product);
        ctx
    }
}

#[test]
fn test_custom_context() {
    let company = Company { name: "ACME", address: "123 Main St" };
    let customer = Customer { name: "John Doe", address: "456 Elm St" };
    let product = Product { name: "Widget", sku: "W123", company: &company, price_in_cents: 10_00 };
    let order = Order { customer: &customer, product: &product };

    // Caching context can work on a read only borrow of the order, prevent us from needing to convert on each access
    let ctx = CachingContext::new(&order);

    let mut tera = Tera::default();

    tera.add_raw_template("shipping_label", "{{ customer.name }}\n{{ customer.address }}")
        .expect("Unable to compile shipping label template");

    tera.add_raw_template("invoice", "Invoice for {{ product.name }}\nSKU: {{ product.sku }}\nPrice: ${{ product.price_in_cents / 100 }}").expect("Unable to compile invoice template");

    assert_eq!(
        tera.render("shipping_label", &ctx).expect("Unable to render shipping label"),
        "John Doe\n456 Elm St"
    );
    assert_eq!(
        tera.render("invoice", &ctx).expect("Unable to render invoice"),
        "Invoice for Widget\nSKU: W123\nPrice: $10"
    );
}
