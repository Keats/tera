use serde_json::Value;
use std::{borrow::Cow, env};
use tera::{Context, ContextProvider, Tera};

#[derive(Clone)]
struct MyContext {
    upper_layer: Context, // for overrides
}

impl MyContext {
    pub fn new() -> Self {
        Self { upper_layer: Context::new() }
    }
}

impl ContextProvider for MyContext {
    fn try_insert<T: serde::Serialize + ?Sized, S: Into<String>>(
        &mut self,
        key: S,
        val: &T,
    ) -> tera::Result<()> {
        self.upper_layer.try_insert(key, val)
    }

    fn find_value(&self, key: &str) -> Option<Cow<Value>> {
        if let Some(val) = self.upper_layer.find_value(key) {
            return Some(val);
        }

        env::var(key.to_uppercase()).map(Value::String).map(Cow::Owned).ok()
    }

    fn find_value_by_dotted_pointer(&self, pointer: &str) -> Option<Cow<Value>> {
        env::var(pointer.to_uppercase().replace('.', "_"))
            .map(Value::String)
            .map(Cow::Owned)
            .ok()
            .or_else(|| self.upper_layer.find_value_by_dotted_pointer(pointer))
    }

    fn into_json(self) -> Value {
        let Value::Object(map) = self.upper_layer.into_json() else { unreachable!() };
        Value::Object(map)
    }
}

fn main() {
    env::set_var("SETTINGS_FOO", "bar");
    let ctx = MyContext::new();

    let output = Tera::one_off("Hello {{ user }}! foo={{ settings.foo }}", &ctx, false).unwrap();

    println!("{output}");
}
