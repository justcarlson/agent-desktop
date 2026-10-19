#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

pub(crate) struct XtestSink;

#[cfg(test)]
#[path = "xtest_sink_tests.rs"]
mod tests;
