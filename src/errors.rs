use serde_json;

error_chain! {
    foreign_links {
        Json(serde_json::Error) #[doc = "An error happened while serializing JSON"];
    }
}
