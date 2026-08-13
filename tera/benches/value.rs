use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use serde_derive::Serialize;

use tera::value::Value;

#[derive(Serialize, Default)]
struct Page {
    path: PathBuf,
    title: String,
    summary: String,
    content: String,
    permalink: String,
    draft: bool,
    generate_feed: bool,
    word_count: usize,
    reading_time: usize,
    backlinks: Vec<String>,
    pages: Vec<Page>,
}

fn realistic_page(i: usize) -> Page {
    Page {
        path: PathBuf::from(format!("/blog/post-{i}/")),
        title: format!("Some blog post title number {i}"),
        summary: "A couple of sentences summarizing what this post is about. ".repeat(4),
        content: format!("<p>Paragraph {i} of rendered HTML content with some length to it.</p>\n")
            .repeat(80),
        permalink: format!("https://example.com/blog/post-{i}/"),
        draft: false,
        generate_feed: true,
        word_count: 1200,
        reading_time: 6,
        backlinks: (0..20).map(|j| format!("/blog/post-{j}/")).collect(),
        pages: Vec::new(),
    }
}

#[derive(Serialize)]
struct Section<'a> {
    title: &'a str,
    description: &'a str,
    permalink: &'a str,
    path: &'a str,
    generate_feed: bool,
    transparent: bool,
    word_count: usize,
    reading_time: usize,
    pages: &'a [Value],
}

fn section(pages: &[Value]) -> Section<'_> {
    Section {
        title: "Blog",
        description: "All the posts on this site",
        permalink: "https://example.com/blog/",
        path: "/blog/",
        generate_feed: true,
        transparent: false,
        word_count: 120_000,
        reading_time: 600,
        pages,
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let pages: Vec<_> = (0..1000).map(realistic_page).collect();

    c.bench_function("from_serializable", |b| {
        b.iter(|| {
            black_box(Value::from_serializable(&pages[1]));
        })
    });

    for n in [10, 100, 1000] {
        c.bench_function(&format!("from_serializable-nested-values-{n}"), |b| {
            let curr_pages: Vec<Value> = (0..n)
                .map(|i| Value::from_serializable(&pages[i]))
                .collect();
            b.iter(|| black_box(Value::from_serializable(&section(&curr_pages))))
        });
    }

    c.bench_function("from_serializable-already-value", |b| {
        let curr_pages: Vec<Value> = (0..100)
            .map(|i| Value::from_serializable(&pages[i]))
            .collect();
        let value = Value::from_serializable(&section(&curr_pages));
        b.iter(|| black_box(Value::from_serializable(&value)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
