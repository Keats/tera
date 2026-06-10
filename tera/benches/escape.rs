use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use tera::escape_html;

const NO_HTML_SHORT: &str = "A paragraph without HTML characters that need to be escaped.";
const HTML_SHORT: &str = "Here->An <<example>> of rust codefn foo(u: &u32) -> &u32 {u}";

// taken from https://github.com/djc/askama/blob/master/askama_escape/benches/all.rs
const NO_HTML_VERY_LONG: &str = r#"
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin scelerisque eu urna in aliquet.
Phasellus ac nulla a urna sagittis consequat id quis est. Nullam eu ex eget erat accumsan dictum
ac lobortis urna. Etiam fermentum ut quam at dignissim. Curabitur vestibulum luctus tellus, sit
amet lobortis augue tempor faucibus. Nullam sed felis eget odio elementum euismod in sit amet massa.
Vestibulum sagittis purus sit amet eros auctor, sit amet pharetra purus dapibus. Donec ornare metus
vel dictum porta. Etiam ut nisl nisi. Nullam rutrum porttitor mi. Donec aliquam ac ipsum eget
hendrerit. Cras faucibus, eros ut pharetra imperdiet, est tellus aliquet felis, eget convallis
lacus ipsum eget quam. Vivamus orci lorem, maximus ac mi eget, bibendum vulputate massa. In
vestibulum dui hendrerit, vestibulum lacus sit amet, posuere erat. Vivamus euismod massa diam,
vulputate euismod lectus vestibulum nec. Donec sit amet massa magna. Nunc ipsum nulla, euismod
quis lacus at, gravida maximus elit. Duis tristique, nisl nullam.
    "#;

const HTML_VERY_LONG: &str = r#"
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris consequat tellus sit
    amet ornare fermentum. Etiam nec erat ante. In at metus a orci mollis scelerisque.
    Sed eget ultrices turpis, at sollicitudin erat. Integer hendrerit nec magna quis
    venenatis. Vivamus non dolor hendrerit, vulputate velit sed, varius nunc. Quisque
    in pharetra mi. Sed ullamcorper nibh malesuada commodo porttitor. Ut scelerisque
    sodales felis quis dignissim. Morbi aliquam finibus justo, sit amet consectetur
    mauris efficitur sit amet. Donec posuere turpis felis, eu lacinia magna accumsan
    quis. Fusce egestas lacus vel fermentum tincidunt. Phasellus a nulla eget lectus
    placerat commodo at eget nisl. Fusce cursus dui quis purus accumsan auctor.
    Donec iaculis felis quis metus consectetur porttitor.
<p>
    Etiam nibh mi, <b>accumsan</b> quis purus sed, posuere fermentum lorem. In pulvinar porta
    maximus. Fusce tincidunt lacinia tellus sit amet tincidunt. Aliquam lacus est, pulvinar
    non metus a, <b>facilisis</b> ultrices quam. Nulla feugiat leo in cursus eleifend. Suspendisse
    eget nisi ac justo sagittis interdum id a ipsum. Nulla mauris justo, scelerisque ac
    rutrum vitae, consequat vel ex.
</p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p></p>
<p>
    Sed sollicitudin <b>sem</b> mauris, at rutrum nibh egestas vel. Ut eu nisi tellus. Praesent dignissim
    orci elementum, mattis turpis eget, maximus ante. Suspendisse luctus eu felis a tempor. Morbi
    ac risus vitae sem molestie ullamcorper. Curabitur ligula augue, sollicitudin quis maximus vel,
    facilisis sed nibh. Aenean auctor magna sem, id rutrum metus convallis quis. Nullam non arcu
    dictum, lobortis erat quis, rhoncus est. Suspendisse venenatis, mi sed venenatis vehicula,
    tortor dolor egestas lectus, et efficitur turpis odio non augue. Integer velit sapien, dictum
    non egestas vitae, hendrerit sed quam. Phasellus a nunc eu erat varius imperdiet. Etiam id
    sollicitudin turpis, vitae molestie orci. Quisque ornare magna quis metus rhoncus commodo.
    Phasellus non mauris velit.
</p>
<p>
    Etiam dictum tellus ipsum, nec varius quam ornare vel. Cras vehicula diam nec sollicitudin
    ultricies. Pellentesque rhoncus sagittis nisl id facilisis. Nunc viverra convallis risus ut
    luctus. Aliquam vestibulum <b>efficitur massa</b>, id tempus nisi posuere a. Aliquam scelerisque
    elit justo. Nullam a ante felis. Cras vitae lorem eu nisi feugiat hendrerit. Maecenas vitae
    suscipit leo, lacinia dignissim lacus. Sed eget volutpat mi. In eu bibendum neque. Pellentesque
    finibus velit a fermentum rhoncus. Maecenas leo purus, eleifend eu lacus a, condimentum sagittis
    justo.
</p>"#;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("escape_html_short", |b| {
        b.iter(|| {
            let mut out: Vec<u8> = Vec::new();
            let _ = escape_html(NO_HTML_SHORT, &mut out);
            let _ = black_box(String::from_utf8_lossy(&out));
            let mut out: Vec<u8> = Vec::new();
            let _ = escape_html(HTML_SHORT, &mut out);
            let _ = black_box(String::from_utf8_lossy(&out));
        })
    });
    c.bench_function("escape_html_long", |b| {
        b.iter(|| {
            let mut out: Vec<u8> = Vec::new();
            let _ = escape_html(NO_HTML_VERY_LONG, &mut out);
            let _ = black_box(String::from_utf8_lossy(&out));
            let mut out: Vec<u8> = Vec::new();
            let _ = escape_html(HTML_VERY_LONG, &mut out);
            let _ = black_box(String::from_utf8_lossy(&out));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
