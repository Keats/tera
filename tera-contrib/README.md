## tera-contrib

Additional functions/filters/tests that require additional dependencies to work.
See docs.rs for available things. Their docstring use the same name as their name in this library
but you can insert them in your Tera instance with whatever name you want.

### Features

Enable only what you need via Cargo features:

| Feature | Provides                                                           |
|---------|--------------------------------------------------------------------|
| `base64` | `b64_encode`, `b64_decode` filters                                 |
| `date` | `now` function, `date` filter, `before`/`after` tests              |
| `filesize_format` | `filesize_format` filter                                           |
| `format` | `format` filter (Rust-like formatting)                             |
| `json` | `json_encode` filter                                               |
| `rand` | `get_random`, `shuffle` functions                                  |
| `regex` | `striptags`, `spaceless`, `regex_replace` filters, `matching` test |
| `slug` | `slug` filter                                                      |
| `urlencode` | `urlencode`, `urlencode_strict` filters                            |

