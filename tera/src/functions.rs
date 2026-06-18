use crate::Value;
use crate::args::Kwargs;
use crate::errors::{Error, TeraResult};
use crate::value::FunctionResult;
use crate::vm::state::State;
use std::sync::Arc;

/// The function function type definition
pub trait Function<Res>: Sync + Send + 'static {
    /// The function type definition
    fn call(&self, kwargs: Kwargs, state: &State) -> Res;

    /// Whether the current function's output should be treated as safe, defaults to `false`
    /// Only needs to be defined if the filter returns a string
    fn is_safe(&self) -> bool {
        false
    }
}

impl<Func, Res> Function<Res> for Func
where
    Func: Fn(Kwargs, &State) -> Res + Sync + Send + 'static,
    Res: FunctionResult,
{
    fn call(&self, kwargs: Kwargs, state: &State) -> Res {
        (self)(kwargs, state)
    }
}

type FunctionFunc = dyn Fn(Kwargs, &State) -> TeraResult<Value> + Sync + Send + 'static;

#[derive(Clone)]
pub(crate) struct StoredFunction {
    func: Arc<FunctionFunc>,
    is_safe: bool,
}

impl StoredFunction {
    pub fn new<Func, Res>(f: Func) -> Self
    where
        Func: Function<Res>,
        Res: FunctionResult,
    {
        let is_safe = f.is_safe();
        let closure = move |kwargs, state: &State| -> TeraResult<Value> {
            f.call(kwargs, state).into_result()
        };

        StoredFunction {
            func: Arc::new(closure),
            is_safe,
        }
    }

    pub fn call(&self, kwargs: Kwargs, state: &State) -> TeraResult<Value> {
        (self.func)(kwargs, state)
    }

    pub fn is_safe(&self) -> bool {
        self.is_safe
    }
}

/// Upper bound on the number of elements `range()` will produce to avoid OOM.
const MAX_RANGE_LEN: usize = 100_000;

pub(crate) fn range(kwargs: Kwargs, _: &State) -> TeraResult<Vec<i128>> {
    let start = kwargs.get::<i128>("start")?.unwrap_or_default();
    let end = kwargs.must_get::<i128>("end")?;
    let step_by = kwargs.get::<i128>("step_by")?.unwrap_or(1);
    if start > end && step_by > 0 {
        return Err(Error::message(
            "Function `range` was called with a `start` argument greater than the `end` one",
        ));
    }
    if step_by == 0 {
        return Err(Error::message(
            "Function `range` was called with a `step_by` argument of 0",
        ));
    }

    let overflow = || Error::message("Function `range` was called with arguments that overflow i128");
    let len = if step_by > 0 {
        let span = end.checked_sub(start).ok_or_else(overflow)?;
        span.checked_add(step_by - 1).ok_or_else(overflow)? / step_by
    } else if start <= end {
        0
    } else {
        let step = step_by.checked_neg().ok_or_else(overflow)?;
        let span = start.checked_sub(end).ok_or_else(overflow)?;
        span.checked_add(step - 1).ok_or_else(overflow)? / step
    };
    if len > MAX_RANGE_LEN as i128 {
        return Err(Error::message(format!(
            "Function `range` would produce {len} elements, which exceeds the limit of {MAX_RANGE_LEN}"
        )));
    }

    let mut values = Vec::with_capacity(len as usize);
    for i in 0..len {
        values.push(start + i * step_by);
    }
    Ok(values)
}

pub(crate) fn throw(kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let message = kwargs.must_get::<&str>("message")?;
    Err(Error::message(message))
}
