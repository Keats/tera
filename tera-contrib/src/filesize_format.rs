use tera::{Kwargs, State, TeraResult};

/// Formats a number of bytes into a human-readable file size string.
/// Uses binary units (KiB, MiB, GiB, etc.) by default but can use decimal.
///
/// ```text
/// {{ num_bytes | filesize_format }}
/// {{ num_bytes | filesize_format(binary=false) }}
/// ```
pub fn filesize_format(val: u64, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let binary = kwargs.get::<bool>("binary")?.unwrap_or(true);

    if binary {
        Ok(humansize::format_size(val, humansize::BINARY))
    } else {
        Ok(humansize::format_size(val, humansize::DECIMAL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tera::value::Map;
    use tera::{Context, Kwargs, State};

    #[test]
    fn test_filesizeformat_binary() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(
            filesize_format(1024, Kwargs::default(), &state).unwrap(),
            "1 KiB"
        );
        assert_eq!(
            filesize_format(1048576, Kwargs::default(), &state).unwrap(),
            "1 MiB"
        );
    }

    #[test]
    fn test_filesizeformat_decimal() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let mut map = Map::new();
        map.insert("binary".into(), false.into());
        let kwargs = Kwargs::new(Arc::new(map));
        assert_eq!(filesize_format(1000, kwargs, &state).unwrap(), "1 kB");
    }

    #[test]
    fn test_register() {
        let mut tera = tera::Tera::default();
        tera.register_filter("filesize_format", filesize_format);
    }
}
