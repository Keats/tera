# Tera

[![Build Status](https://travis-ci.org/Keats/tera.svg)](https://travis-ci.org/Keats/tera)


Roadmap:
- if/for rendering
- inherit/block tags & rendering
- make it work on stable (while still using serde rather than rustc-serialize)
- error handling
- make it the easiest possible for the context (for example Context could be exposed with a `add` method that takes care of serializing to avoid the user dealing with to_value)
- filters
- ignore whitespace/newlines on tags

Other:
- move to gitlab once CI is figured out
