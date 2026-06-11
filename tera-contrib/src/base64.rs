use base64::{Engine, engine::general_purpose};
use tera::{Kwargs, State, TeraResult};

const STANDARD_DECODE: general_purpose::GeneralPurpose = general_purpose::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

const URL_SAFE_DECODE: general_purpose::GeneralPurpose = general_purpose::GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

/// Encodes a string to base64.
/// Takes an optional `url_safe` bool parameter (`false` by default) if you want to use the URL SAFE b64 characters
/// and a `padded` bool parameter on whether you want padding (`true` by default).

/// ```text
/// {{ value | b64_encode }}
/// {{ value | b64_encode(url_safe=true) }}
/// {{ value | b64_encode(url_safe=true, padded=false) }}
/// ```
pub fn b64_encode(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let url_safe = kwargs.get::<bool>("url_safe")?.unwrap_or(false);
    let padded = kwargs.get::<bool>("padded")?.unwrap_or(true);
    let encoded = match (url_safe, padded) {
        (false, true) => general_purpose::STANDARD.encode(val),
        (false, false) => general_purpose::STANDARD_NO_PAD.encode(val),
        (true, true) => general_purpose::URL_SAFE.encode(val),
        (true, false) => general_purpose::URL_SAFE_NO_PAD.encode(val),
    };
    Ok(encoded)
}

/// Decodes a base64 string.
///
/// ```text
/// {{ value | b64_decode }}
/// {{ value | b64_decode(url_safe=true) }}
/// ```
pub fn b64_decode(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let url_safe = kwargs.get::<bool>("url_safe")?.unwrap_or(false);
    let decoded = if url_safe {
        URL_SAFE_DECODE.decode(val)
    } else {
        STANDARD_DECODE.decode(val)
    };
    let bytes = decoded.map_err(|e| tera::Error::message(format!("Invalid base64: {e}")))?;
    String::from_utf8(bytes).map_err(|e| tera::Error::message(format!("Invalid UTF-8: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tera::value::Map;
    use tera::{Context, Kwargs, State};

    #[test]
    fn test_b64_encode() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(
            b64_encode("hello world", Kwargs::default(), &state).unwrap(),
            "aGVsbG8gd29ybGQ="
        );
    }

    #[test]
    fn test_b64_encode_no_padding() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let mut kwargs_map = Map::new();
        kwargs_map.insert("padded".into(), false.into());
        let kwargs = Kwargs::new(Arc::new(kwargs_map));
        assert_eq!(
            b64_encode("hello world", kwargs, &state).unwrap(),
            "aGVsbG8gd29ybGQ"
        );
    }

    #[test]
    fn test_b64_encode_url_safe() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let mut kwargs_map = Map::new();
        kwargs_map.insert("url_safe".into(), true.into());
        let kwargs = Kwargs::new(Arc::new(kwargs_map));
        // String with characters that differ between standard and URL-safe
        assert_eq!(b64_encode("<<??>>", kwargs, &state).unwrap(), "PDw_Pz4-");
    }

    #[test]
    fn test_b64_decode() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(
            b64_decode("aGVsbG8gd29ybGQ=", Kwargs::default(), &state).unwrap(),
            "hello world"
        );
        // works as well without padding
        assert_eq!(
            b64_decode("aGVsbG8gd29ybGQ", Kwargs::default(), &state).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_b64_decode_url_safe() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let mut kwargs_map = Map::new();
        kwargs_map.insert("url_safe".into(), true.into());
        let kwargs = Kwargs::new(Arc::new(kwargs_map));
        assert_eq!(b64_decode("PDw_Pz4-", kwargs, &state).unwrap(), "<<??>>");
    }

    #[test]
    fn test_b64_roundtrip() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let original = "Hello, 世界! 🌍";
        let encoded = b64_encode(original, Kwargs::default(), &state).unwrap();
        let decoded = b64_decode(&encoded, Kwargs::default(), &state).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_b64_decode_invalid() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let result = b64_decode("not valid base64!!!", Kwargs::default(), &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_register() {
        let mut tera = tera::Tera::default();
        tera.register_filter("b64_encode", b64_encode);
        tera.register_filter("b64_decode", b64_decode);
    }
}
