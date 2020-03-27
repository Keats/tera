use crate::context::Context;
use crate::tera::Tera;

#[test]
fn render_simple_inheritance() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("top", "{% block pre %}{% endblock pre %}{% block main %}{% endblock main %}"),
        ("bottom", "{% extends \"top\" %}{% block main %}MAIN{% endblock %}"),
    ])
    .unwrap();
    let result = tera.render("bottom", &Context::new());

    assert_eq!(result.unwrap(), "MAIN".to_string());
}

#[test]
fn render_simple_inheritance_super() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("top", "{% block main %}TOP{% endblock main %}"),
        ("bottom", "{% extends \"top\" %}{% block main %}{{ super() }}MAIN{% endblock %}"),
    ])
    .unwrap();
    let result = tera.render("bottom", &Context::new());

    assert_eq!(result.unwrap(), "TOPMAIN".to_string());
}

#[test]
fn render_multiple_inheritance() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("top", "{% block pre %}{% endblock pre %}{% block main %}{% endblock main %}"),
        ("mid", "{% extends \"top\" %}{% block pre %}PRE{% endblock pre %}"),
        ("bottom", "{% extends \"mid\" %}{% block main %}MAIN{% endblock main %}"),
    ])
    .unwrap();
    let result = tera.render("bottom", &Context::new());

    assert_eq!(result.unwrap(), "PREMAIN".to_string());
}

#[test]
fn render_multiple_inheritance_with_super() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        (
            "grandparent",
            "{% block hey %}hello{% endblock hey %} {% block ending %}sincerely{% endblock ending %}",
        ),
        (
            "parent",
            "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }}{% endblock hey %}",
        ),
        (
            "child",
            "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}",
        ),
    ]).unwrap();
    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap(),
        "dad says hi and grandma says hello sincerely with love".to_string()
    );
}

#[test]
fn render_filter_section_inheritance_no_override() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("top", "{% filter upper %}hello {% block main %}top{% endblock main %}{% endfilter %}"),
        ("bottom", "{% extends 'top' %}"),
    ])
    .unwrap();
    let result = tera.render("bottom", &Context::new());

    assert_eq!(result.unwrap(), "HELLO TOP".to_string());
}

#[test]
fn render_filter_section_inheritance() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("top", "{% filter upper %}hello {% block main %}top{% endblock main %}{% endfilter %}"),
        ("bottom", "{% extends 'top' %}{% block main %}bottom{% endblock %}"),
    ])
    .unwrap();
    let result = tera.render("bottom", &Context::new());

    assert_eq!(result.unwrap(), "HELLO BOTTOM".to_string());
}

#[test]
fn render_super_multiple_inheritance_nested_block() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        (
            "grandparent",
            "{% block hey %}hello{% endblock hey %}",
        ),
        (
            "parent",
            "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}",
        ),
        (
            "child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}",
        ),
    ]).unwrap();
    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap(),
        "dad says hi and grandma says hello sincerely with love".to_string()
    );
}

#[test]
fn render_nested_block_multiple_inheritance_no_super() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("index", "{% block content%}INDEX{% endblock content %}"),
        (
            "docs",
            "{% extends \"index\" %}{% block content%}DOCS{% block more %}MORE{% endblock more %}{% endblock content %}",
        ),
        ("page", "{% extends \"docs\" %}{% block more %}PAGE{% endblock more %}"),
    ]).unwrap();

    let result = tera.render("page", &Context::new());

    assert_eq!(result.unwrap(), "DOCSPAGE".to_string());
}

#[test]
fn render_super_in_top_block_errors() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("index", "{% block content%}{{super()}}{% endblock content %}")])
        .unwrap();

    let result = tera.render("index", &Context::new());
    assert!(result.is_err());
}

// https://github.com/Keats/tera/issues/215
#[test]
fn render_super_in_grandchild_without_redefining_works() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("grandparent", "{% block title %}Title{% endblock %}"),
        (
            "parent",
            "{% extends \"grandparent\" %}{% block title %}{{ super() }} - More{% endblock %}",
        ),
        ("child", "{% extends \"parent\" %}"),
    ])
    .unwrap();

    let result = tera.render("child", &Context::new());
    assert_eq!(result.unwrap(), "Title - More".to_string());
}

#[test]
fn render_super_in_grandchild_without_redefining_in_parent_works() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("grandparent", "{% block title %}Title{% endblock %}"),
        ("parent", "{% extends \"grandparent\" %}"),
        ("child", "{% extends \"parent\" %}{% block title %}{{ super() }} - More{% endblock %}"),
    ])
    .unwrap();

    let result = tera.render("child", &Context::new());
    assert_eq!(result.unwrap(), "Title - More".to_string());
}
