/// Tests for the `system_variable_not_found_fallback` feature.
///
/// When a template sets `{% set system_variable_not_found_fallback = '<value>' %}`,
/// undefined variables are replaced with that value instead of causing an error.
/// Without that variable, normal error behavior is preserved.
/// The `default` filter takes precedence over the system fallback.
use crate::context::Context;
use crate::tera::Tera;

/// Renders a template with the fallback variable prepended.
fn render_with_fallback(template: &str, ctx: &Context, fallback: &str) -> String {
    let mut tera = Tera::default();
    let full =
        format!("{{% set system_variable_not_found_fallback = '{}' %}}{}", fallback, template);
    tera.add_raw_template("tpl", &full).unwrap();
    tera.render("tpl", ctx).unwrap()
}

/// Renders a template without the fallback variable; returns Result so callers can assert on errors.
fn render_no_fallback(template: &str, ctx: &Context) -> Result<String, crate::errors::Error> {
    let mut tera = Tera::default();
    tera.add_raw_template("tpl", template).unwrap();
    tera.render("tpl", ctx)
}

// ---------------------------------------------------------------------------
// Basic fallback behaviour
// ---------------------------------------------------------------------------

#[test]
fn test_basic_undefined_variable_replaced() {
    let ctx = Context::new();
    let result = render_with_fallback("{{ missing }}", &ctx, "N/A");
    assert_eq!(result, "N/A");
}

#[test]
fn test_defined_variable_not_affected() {
    let mut ctx = Context::new();
    ctx.insert("name", "world");
    let result = render_with_fallback("Hello {{ name }}!", &ctx, "N/A");
    assert_eq!(result, "Hello world!");
}

#[test]
fn test_multiple_undefined_variables() {
    let ctx = Context::new();
    let result = render_with_fallback("{{ a }} {{ b }} {{ c }}", &ctx, "?");
    assert_eq!(result, "? ? ?");
}

#[test]
fn test_mix_of_defined_and_undefined() {
    let mut ctx = Context::new();
    ctx.insert("defined", "yes");
    let result = render_with_fallback("{{ defined }}/{{ undefined }}", &ctx, "no");
    assert_eq!(result, "yes/no");
}

// ---------------------------------------------------------------------------
// No fallback → normal error behavior preserved
// ---------------------------------------------------------------------------

#[test]
fn test_no_fallback_undefined_variable_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{{ missing }}", &ctx);
    assert!(result.is_err(), "expected an error for undefined variable without fallback");
}

#[test]
fn test_no_fallback_defined_variable_renders() {
    let mut ctx = Context::new();
    ctx.insert("name", "world");
    let result = render_no_fallback("Hello {{ name }}!", &ctx).unwrap();
    assert_eq!(result, "Hello world!");
}

// ---------------------------------------------------------------------------
// Default filter takes precedence
// ---------------------------------------------------------------------------

#[test]
fn test_default_filter_takes_precedence_over_fallback() {
    let ctx = Context::new();
    // The `default` filter should win over the system fallback
    let result = render_with_fallback("{{ missing | default(value='explicit') }}", &ctx, "system");
    assert_eq!(result, "explicit");
}

#[test]
fn test_default_filter_on_defined_variable() {
    let mut ctx = Context::new();
    ctx.insert("val", "real");
    let result = render_with_fallback("{{ val | default(value='explicit') }}", &ctx, "system");
    assert_eq!(result, "real");
}

// ---------------------------------------------------------------------------
// {% set %} with undefined variable
// ---------------------------------------------------------------------------

#[test]
fn test_set_from_undefined_gets_fallback() {
    let ctx = Context::new();
    let result = render_with_fallback("{% set x = missing %}{{ x }}", &ctx, "fallback");
    assert_eq!(result, "fallback");
}

#[test]
fn test_set_from_defined_unaffected() {
    let mut ctx = Context::new();
    ctx.insert("src", "hello");
    let result = render_with_fallback("{% set x = src %}{{ x }}", &ctx, "fallback");
    assert_eq!(result, "hello");
}

#[test]
fn test_set_from_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{% set x = missing %}{{ x }}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_nested_set_chain() {
    let ctx = Context::new();
    // x = missing → fallback; y = x → "fallback"
    let result = render_with_fallback("{% set x = missing %}{% set y = x %}{{ y }}", &ctx, "fb");
    assert_eq!(result, "fb");
}

// ---------------------------------------------------------------------------
// {% if %} blocks
// ---------------------------------------------------------------------------

#[test]
fn test_if_undefined_condition_treats_as_falsy() {
    let ctx = Context::new();
    // Undefined in a boolean context is already silently falsy (pre-existing behaviour)
    let result = render_with_fallback("{% if missing %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_if_eq_undefined_vs_string_is_false() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% if missing == 'hello' %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_if_not_eq_undefined_vs_string_is_true() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% if missing != 'hello' %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "yes");
}

#[test]
fn test_if_eq_undefined_no_fallback_is_false() {
    let ctx = Context::new();
    let result = render_no_fallback("{% if missing %}yes{% else %}no{% endif %}", &ctx);
    assert_eq!(result.unwrap(), "no");
}

#[test]
fn test_if_eq_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{% if missing == 'hello' %}yes{% else %}no{% endif %}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_if_defined_eq_works_normally() {
    let mut ctx = Context::new();
    ctx.insert("val", "hello");
    let result =
        render_with_fallback("{% if val == 'hello' %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "yes");
}

#[test]
fn test_if_undefined_in_list_is_false() {
    let ctx = Context::new();
    let result = render_with_fallback(
        "{% if missing in ['a', 'b'] %}yes{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "no");
}

#[test]
fn test_if_undefined_not_in_list_is_true() {
    let ctx = Context::new();
    let result = render_with_fallback(
        "{% if missing not in ['a', 'b'] %}yes{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "yes");
}

#[test]
fn test_if_value_in_undefined_list_is_false() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% if 'a' in missing_list %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_if_value_not_in_undefined_list_is_true() {
    let ctx = Context::new();
    let result = render_with_fallback(
        "{% if 'a' not in missing_list %}yes{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "yes");
}

// ---------------------------------------------------------------------------
// {% for %} loops
// ---------------------------------------------------------------------------

#[test]
fn test_for_loop_undefined_container_skips_body() {
    let ctx = Context::new();
    let result = render_with_fallback(
        "{% for item in missing_list %}{{ item }}{% endfor %}done",
        &ctx,
        "fb",
    );
    assert_eq!(result, "done");
}

#[test]
fn test_for_loop_undefined_container_renders_else_body() {
    let ctx = Context::new();
    let result = render_with_fallback(
        "{% for item in missing_list %}{{ item }}{% else %}empty{% endfor %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "empty");
}

#[test]
fn test_for_loop_undefined_container_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{% for item in missing_list %}{{ item }}{% endfor %}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_for_loop_defined_container_works() {
    let mut ctx = Context::new();
    ctx.insert("items", &["a", "b", "c"]);
    let result = render_with_fallback("{% for item in items %}{{ item }}{% endfor %}", &ctx, "fb");
    assert_eq!(result, "abc");
}

#[test]
fn test_for_loop_body_undefined_variable_replaced() {
    let mut ctx = Context::new();
    ctx.insert("items", &[1, 2]);
    // `extra` is not in ctx; it should get the fallback inside the loop body
    let result = render_with_fallback(
        "{% for item in items %}{{ item }}:{{ extra }} {% endfor %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "1:fb 2:fb ");
}

// ---------------------------------------------------------------------------
// Scoping: fallback defined at template level is visible inside for-loops
// ---------------------------------------------------------------------------

#[test]
fn test_fallback_visible_inside_for_loop() {
    let mut ctx = Context::new();
    ctx.insert("items", &[1u32, 2, 3]);
    let result =
        render_with_fallback("{% for item in items %}{{ missing }}{% endfor %}", &ctx, "X");
    assert_eq!(result, "XXX");
}

#[test]
fn test_missing_inside_loop_no_fallback_fails() {
    let mut ctx = Context::new();
    ctx.insert("items", &[1u32, 2, 3]);
    let result = render_no_fallback("{% for item in items %}{{ missing }}{% endfor %}", &ctx);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Fallback value types
// ---------------------------------------------------------------------------

#[test]
fn test_fallback_empty_string() {
    let ctx = Context::new();
    let result = render_with_fallback("a{{ missing }}b", &ctx, "");
    assert_eq!(result, "ab");
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_nested_undefined_in_object_access() {
    let ctx = Context::new();
    // obj.field — obj is missing entirely
    let result = render_with_fallback("{{ obj.field }}", &ctx, "N/A");
    assert_eq!(result, "N/A");
}

#[test]
fn test_defined_and_undefined_in_same_if_block() {
    let mut ctx = Context::new();
    ctx.insert("a", "hello");
    // b is undefined; a == 'hello' is true, so renders true branch with fallback
    let result =
        render_with_fallback("{% if a == 'hello' %}{{ b }}{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "fb");
}

#[test]
fn test_set_global_from_undefined_gets_fallback() {
    let ctx = Context::new();
    // `set_global` assigns to the top-level frame
    let result = render_with_fallback("{% set_global x = missing %}{{ x }}", &ctx, "global_fb");
    assert_eq!(result, "global_fb");
}

// ---------------------------------------------------------------------------
// String concatenation with undefined variables
// ---------------------------------------------------------------------------

#[test]
fn test_string_concat_undefined_gives_fallback_for_whole_expr() {
    let ctx = Context::new();
    // The entire concatenation expression falls back; the prefix is not preserved
    let result = render_with_fallback(r#"{{ "prefix-" ~ missing }}"#, &ctx, "N/A");
    assert_eq!(result, "N/A");
}

#[test]
fn test_string_concat_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback(r#"{{ "prefix-" ~ missing }}"#, &ctx);
    assert!(result.is_err());
}

#[test]
fn test_string_concat_all_defined_unaffected() {
    let mut ctx = Context::new();
    ctx.insert("a", "hello");
    ctx.insert("b", "world");
    let result = render_with_fallback(r#"{{ a ~ "-" ~ b }}"#, &ctx, "N/A");
    assert_eq!(result, "hello-world");
}

#[test]
fn test_string_concat_defined_and_undefined_gives_fallback() {
    let mut ctx = Context::new();
    ctx.insert("a", "real");
    // concatenation with undefined — whole expression falls back
    let result = render_with_fallback(r#"{{ a ~ "-" ~ missing }}"#, &ctx, "fb");
    assert_eq!(result, "fb");
}

// ---------------------------------------------------------------------------
// Filters applied to undefined variables
// ---------------------------------------------------------------------------

#[test]
fn test_filter_on_undefined_gives_fallback() {
    let ctx = Context::new();
    // The ident lookup fails before the filter runs; the whole expression falls back
    let result = render_with_fallback("{{ missing | lower }}", &ctx, "N/A");
    assert_eq!(result, "N/A");
}

#[test]
fn test_filter_on_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{{ missing | upper }}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_filter_on_defined_var_works() {
    let mut ctx = Context::new();
    ctx.insert("name", "world");
    let result = render_with_fallback("{{ name | upper }}", &ctx, "N/A");
    assert_eq!(result, "WORLD");
}

#[test]
fn test_default_filter_beats_other_filters() {
    // default comes first in the filter chain, so it wins before `upper` is applied
    let ctx = Context::new();
    let result =
        render_with_fallback("{{ missing | default(value='explicit') | upper }}", &ctx, "system");
    assert_eq!(result, "EXPLICIT");
}

// ---------------------------------------------------------------------------
// Numeric comparisons (>, >=, <, <=) with undefined variables
// ---------------------------------------------------------------------------

#[test]
fn test_numeric_gt_undefined_with_fallback_is_false() {
    let ctx = Context::new();
    let result = render_with_fallback("{% if missing > 5 %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_numeric_lt_undefined_with_fallback_is_false() {
    let ctx = Context::new();
    let result = render_with_fallback("{% if missing < 5 %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_numeric_gte_undefined_with_fallback_is_false() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% if missing >= 5 %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_numeric_lte_undefined_with_fallback_is_false() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% if missing <= 5 %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "no");
}

#[test]
fn test_numeric_gt_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback("{% if missing > 5 %}yes{% else %}no{% endif %}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_numeric_comparison_defined_values_work() {
    let mut ctx = Context::new();
    ctx.insert("score", &10i64);
    let result = render_with_fallback("{% if score > 5 %}high{% else %}low{% endif %}", &ctx, "fb");
    assert_eq!(result, "high");
}

// ---------------------------------------------------------------------------
// Complex logical expressions
// ---------------------------------------------------------------------------

#[test]
fn test_and_with_undefined_eq_and_defined_true_is_false() {
    // (missing == "x") and true  →  false and true  →  false
    let mut ctx = Context::new();
    ctx.insert("flag", &true);
    let result = render_with_fallback(
        "{% if missing == 'x' and flag %}yes{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "no");
}

#[test]
fn test_and_with_undefined_eq_no_fallback_errors() {
    let mut ctx = Context::new();
    ctx.insert("flag", &true);
    let result =
        render_no_fallback("{% if missing == 'x' and flag %}yes{% else %}no{% endif %}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_or_with_undefined_eq_and_defined_true_is_true() {
    // false == "x"  or  true  →  false or true  →  true (rhs not short-circuited here)
    // Actually: (missing == "x") or true:
    //   eval_as_bool(Eq(missing,"x")) → false (with fallback)
    //   false || eval_as_bool(Ident(true_flag)) → true
    let mut ctx = Context::new();
    ctx.insert("flag", &true);
    let result = render_with_fallback(
        "{% if missing == 'x' or flag %}yes{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "yes");
}

#[test]
fn test_or_short_circuit_avoids_undefined_error() {
    // true or missing_eq — lhs is true, rhs not evaluated; no error even without fallback
    let mut ctx = Context::new();
    ctx.insert("flag", &true);
    let result =
        render_no_fallback("{% if flag or missing == 'x' %}yes{% else %}no{% endif %}", &ctx);
    // Rust short-circuits: true || ... does not evaluate rhs
    assert_eq!(result.unwrap(), "yes");
}

#[test]
fn test_and_short_circuit_avoids_undefined_error() {
    // false and missing_eq — lhs is false, rhs not evaluated; no error even without fallback
    let mut ctx = Context::new();
    ctx.insert("flag", &false);
    let result =
        render_no_fallback("{% if flag and missing == 'x' %}yes{% else %}no{% endif %}", &ctx);
    assert_eq!(result.unwrap(), "no");
}

#[test]
fn test_complex_condition_multiple_undefined() {
    let ctx = Context::new();
    // (a == "x") or (b != "y")  →  false or true  →  true
    let result =
        render_with_fallback("{% if a == 'x' or b != 'y' %}yes{% else %}no{% endif %}", &ctx, "fb");
    assert_eq!(result, "yes");
}

// ---------------------------------------------------------------------------
// Nested for loops
// ---------------------------------------------------------------------------

#[test]
fn test_nested_for_loop_inner_container_undefined_skipped() {
    let mut ctx = Context::new();
    ctx.insert("rows", &[1u32, 2]);
    // inner loop container is missing — body skipped for each outer iteration
    let result = render_with_fallback(
        "{% for row in rows %}[{% for cell in missing_cells %}{{ cell }}{% endfor %}]{% endfor %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "[][]");
}

#[test]
fn test_nested_for_loop_inner_container_undefined_no_fallback_errors() {
    let mut ctx = Context::new();
    ctx.insert("rows", &[1u32, 2]);
    let result = render_no_fallback(
        "{% for row in rows %}{% for cell in missing_cells %}{{ cell }}{% endfor %}{% endfor %}",
        &ctx,
    );
    assert!(result.is_err());
}

#[test]
fn test_nested_for_loop_undefined_var_in_body() {
    let mut ctx = Context::new();
    ctx.insert("matrix", &vec![vec![1u32, 2], vec![3, 4]]);
    // `label` is undefined; gets replaced with fallback in inner body
    let result = render_with_fallback(
        "{% for row in matrix %}{% for cell in row %}{{ cell }}:{{ label }} {% endfor %}{% endfor %}",
        &ctx,
        "?",
    );
    assert_eq!(result, "1:? 2:? 3:? 4:? ");
}

#[test]
fn test_nested_for_with_if_on_undefined_field() {
    let mut ctx = Context::new();
    ctx.insert("items", &["a", "b"]);
    // `priority` is undefined — eq comparison returns false with fallback
    let result = render_with_fallback(
        r#"{% for item in items %}{% if item == priority %}!{% else %}{{ item }}{% endif %}{% endfor %}"#,
        &ctx,
        "fb",
    );
    assert_eq!(result, "ab");
}

// ---------------------------------------------------------------------------
// Deeply chained set assignments
// ---------------------------------------------------------------------------

#[test]
fn test_deep_set_chain_all_undefined() {
    let ctx = Context::new();
    let result =
        render_with_fallback("{% set a = x %}{% set b = a %}{% set c = b %}{{ c }}", &ctx, "deep");
    assert_eq!(result, "deep");
}

#[test]
fn test_set_then_use_in_condition() {
    let ctx = Context::new();
    // x = missing → fallback; then x == fallback is true
    let result = render_with_fallback(
        "{% set x = missing %}{% if x == 'fb' %}match{% else %}no{% endif %}",
        &ctx,
        "fb",
    );
    assert_eq!(result, "match");
}

#[test]
fn test_set_global_inside_for_loop_from_undefined() {
    let mut ctx = Context::new();
    ctx.insert("items", &[1u32, 2, 3]);
    // `missing_val` is undefined; set_global captures fallback in each iteration
    let result = render_with_fallback(
        "{% for item in items %}{% set_global last = missing_val %}{% endfor %}{{ last }}",
        &ctx,
        "done",
    );
    assert_eq!(result, "done");
}

// ---------------------------------------------------------------------------
// Undefined property access on objects
// ---------------------------------------------------------------------------

#[test]
fn test_defined_object_missing_field_falls_back() {
    let mut ctx = Context::new();
    ctx.insert("user", &serde_json::json!({"name": "Alice"}));
    // `user.age` field doesn't exist
    let result = render_with_fallback("{{ user.age }}", &ctx, "unknown");
    assert_eq!(result, "unknown");
}

#[test]
fn test_defined_object_missing_field_no_fallback_errors() {
    let mut ctx = Context::new();
    ctx.insert("user", &serde_json::json!({"name": "Alice"}));
    let result = render_no_fallback("{{ user.age }}", &ctx);
    assert!(result.is_err());
}

#[test]
fn test_defined_object_present_field_unaffected() {
    let mut ctx = Context::new();
    ctx.insert("user", &serde_json::json!({"name": "Alice", "age": 30}));
    let result = render_with_fallback("{{ user.name }} {{ user.age }}", &ctx, "N/A");
    assert_eq!(result, "Alice 30");
}

#[test]
fn test_deeply_nested_field_access_missing() {
    let mut ctx = Context::new();
    ctx.insert("a", &serde_json::json!({"b": {"c": "deep"}}));
    // a.b.c exists; a.b.d does not
    let result = render_with_fallback("{{ a.b.c }}-{{ a.b.d }}", &ctx, "N/A");
    assert_eq!(result, "deep-N/A");
}

// ---------------------------------------------------------------------------
// Template isolation: fallback in one render does not affect another
// ---------------------------------------------------------------------------

#[test]
fn test_fallback_does_not_bleed_between_renders() {
    let mut tera = Tera::default();
    tera.add_raw_template(
        "with_fallback",
        "{% set system_variable_not_found_fallback = 'fb' %}{{ missing }}",
    )
    .unwrap();
    tera.add_raw_template("without_fallback", "{{ missing }}").unwrap();

    let ctx = Context::new();
    // First render succeeds with fallback
    let r1 = tera.render("with_fallback", &ctx).unwrap();
    assert_eq!(r1, "fb");
    // Second render of template without fallback still errors
    let r2 = tera.render("without_fallback", &ctx);
    assert!(r2.is_err());
}

// ---------------------------------------------------------------------------
// Mixed complex scenarios
// ---------------------------------------------------------------------------

#[test]
fn test_full_notification_template_partial_context() {
    // Simulates a real-world notification template where some fields may be absent
    let mut ctx = Context::new();
    ctx.insert("alert_name", "CPU High");
    ctx.insert("severity", "critical");
    // `description`, `runbook_url`, `threshold` are missing
    let tpl = r#"
Alert: {{ alert_name }}
Severity: {{ severity }}
Description: {{ description }}
Threshold: {{ threshold }}
Runbook: {{ runbook_url }}
{% if threshold > 90 %}Above limit{% else %}Check manually{% endif %}
"#;
    let result = render_with_fallback(tpl, &ctx, "N/A");
    assert!(result.contains("Alert: CPU High"));
    assert!(result.contains("Description: N/A"));
    assert!(result.contains("Threshold: N/A"));
    assert!(result.contains("Runbook: N/A"));
    assert!(result.contains("Check manually")); // threshold is undefined → > 90 is false
}

#[test]
fn test_for_loop_with_conditional_on_undefined_field() {
    // Loop over defined items; each item may or may not have an optional field
    let mut ctx = Context::new();
    ctx.insert(
        "events",
        &vec![
            serde_json::json!({"name": "login", "user": "alice"}),
            serde_json::json!({"name": "logout"}), // no "user" field
        ],
    );
    let tpl = r#"{% for event in events %}{{ event.name }}({{ event.user }}) {% endfor %}"#;
    let result = render_with_fallback(tpl, &ctx, "anonymous");
    assert_eq!(result, "login(alice) logout(anonymous) ");
}

#[test]
fn test_for_loop_with_conditional_on_undefined_field_no_fallback_errors() {
    let mut ctx = Context::new();
    ctx.insert(
        "events",
        &vec![
            serde_json::json!({"name": "login", "user": "alice"}),
            serde_json::json!({"name": "logout"}),
        ],
    );
    let tpl = r#"{% for event in events %}{{ event.name }}({{ event.user }}) {% endfor %}"#;
    let result = render_no_fallback(tpl, &ctx);
    assert!(result.is_err());
}

#[test]
fn test_if_elseif_chain_all_undefined() {
    let ctx = Context::new();
    let result = render_with_fallback(
        r#"{% if a == 'x' %}A{% elif b == 'y' %}B{% elif c == 'z' %}C{% else %}none{% endif %}"#,
        &ctx,
        "fb",
    );
    // All conditions: undefined != "x", undefined != "y", undefined != "z" → else branch
    assert_eq!(result, "none");
}

#[test]
fn test_if_elseif_chain_all_undefined_no_fallback_errors() {
    let ctx = Context::new();
    let result = render_no_fallback(
        r#"{% if a == 'x' %}A{% elif b == 'y' %}B{% else %}none{% endif %}"#,
        &ctx,
    );
    assert!(result.is_err());
}

#[test]
fn test_set_with_default_filter_takes_precedence_in_assignment() {
    let ctx = Context::new();
    // Even inside {% set %}, default filter beats system fallback
    let result = render_with_fallback(
        "{% set x = missing | default(value='explicit') %}{{ x }}",
        &ctx,
        "system",
    );
    assert_eq!(result, "explicit");
}
