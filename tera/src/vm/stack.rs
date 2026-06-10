use crate::Value;
use std::ops::RangeInclusive;

/// A range of span indices (inclusive) for error reporting.
/// When an error occurs, spans can be looked up from the chunk and expanded.
pub(crate) type SpanRange = RangeInclusive<u32>;

/// Combine two span ranges into one that covers both
#[inline]
pub(crate) fn combine_spans(first: &SpanRange, second: &SpanRange) -> SpanRange {
    *first.start().min(second.start())..=*first.end().max(second.end())
}

#[derive(Debug, Eq, PartialEq, Default)]
pub(crate) struct Stack {
    values: Vec<(Value, SpanRange)>,
}

impl Stack {
    pub(crate) fn new() -> Self {
        Self {
            values: Vec::with_capacity(64),
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, val: Value, span: SpanRange) {
        self.values.push((val, span));
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> (Value, SpanRange) {
        self.values.pop().expect("to have a value")
    }

    #[inline]
    pub(crate) fn peek(&self) -> &(Value, SpanRange) {
        self.values.last().expect("to peek a value")
    }

    /// Only used by list comprehension to avoid pop + push
    #[inline]
    pub(crate) fn peek_mut(&mut self) -> &mut (Value, SpanRange) {
        self.values.last_mut().expect("to peek a value")
    }
}
