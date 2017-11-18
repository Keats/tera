extern crate tera;
extern crate hyper;

// Doesn't do anything but ensures https://github.com/Keats/tera/issues/175 works
#[allow(dead_code)]
struct HttpHandler {
    templates: tera::Tera,
    http_client: hyper::Client,
}

impl hyper::server::Handler for HttpHandler {
    fn handle(&self, _: hyper::server::Request, _: hyper::server::Response) {
        // ...
    }
}

fn main() {}

