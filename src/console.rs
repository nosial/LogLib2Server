use crate::event::{ExceptionDetails, LogEvent, StackTrace};
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct ConsoleConfig {
    pub color: bool,
    pub trace_format: TraceFormatKind,
    pub always_show_stacktraces: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFormatKind {
    None,
    Basic,
    Full,
}

impl ConsoleConfig {
    pub const fn new(
        color: bool,
        trace_format: TraceFormatKind,
        always_show_stacktraces: bool,
    ) -> Self {
        Self {
            color,
            trace_format,
            always_show_stacktraces,
        }
    }
}

fn level_color(level: &str) -> &'static str {
    match level {
        "DBG" => "\x1b[36m",
        "VRB" => "\x1b[34m",
        "INFO" => "\x1b[32m",
        "WRN" => "\x1b[33m",
        "ERR" => "\x1b[31m",
        "CRT" => "\x1b[31;47m",
        _ => "\x1b[0m",
    }
}

const fn reset() -> &'static str {
    "\x1b[0m"
}

fn bold(s: &str) -> String {
    format!("\x1b[1m{s}\x1b[0m")
}

pub fn format_event(event: &LogEvent, config: &ConsoleConfig) -> (String, bool) {
    let level = event.level_value();
    let is_error = matches!(level, "WRN" | "ERR" | "CRT");
    let mut output = String::new();

    if config.color {
        let _ = write!(output, "{}", level_color(level));
    }

    if config.color {
        let _ = write!(output, "[{}]", bold(&event.application_name));
    } else {
        let _ = write!(output, "[{}]", event.application_name);
    }

    let _ = write!(output, " {}", event.message);

    if config.color {
        let _ = write!(output, "{}", reset());
    }

    if let Some(ref exc) = event.exception {
        let _ = write!(output, "{}", format_exception(exc, config));
    }

    let st = event
        .traces
        .as_ref()
        .and_then(|t| if t.is_empty() { None } else { Some(t) });

    if config.always_show_stacktraces {
        if let Some(traces) = st {
            let _ = write!(output, "\n  Stack Trace:");
            for trace in traces {
                let _ = write!(output, "\n    {}", format_trace_basic(trace));
            }
        }
    }

    (output, is_error)
}

fn format_exception(exc: &ExceptionDetails, config: &ConsoleConfig) -> String {
    let mut out = String::new();

    let _ = writeln!(out);
    if config.color {
        let _ = write!(out, "\x1b[31m");
    }
    let _ = write!(out, "{}", exc.name);

    if let Some(code) = exc.code {
        if code != 0 {
            let _ = write!(out, " ({code})");
        }
    }

    let _ = write!(out, ": {}", exc.message);

    if let (Some(ref file), Some(line)) = (&exc.file, exc.line) {
        let _ = write!(out, "\n  File: {file}:{line}");
    } else if let Some(ref file) = exc.file {
        let _ = write!(out, "\n  File: {file}");
    }

    if config.color {
        let _ = write!(out, "{}", reset());
    }

    if let Some(ref trace) = exc.trace {
        if !trace.is_empty() {
            let _ = write!(out, "\n  Stack Trace:");
            for frame in trace {
                let _ = write!(out, "\n    - {}", format_stack_trace(frame, config));
            }
        }
    }

    if let Some(ref prev) = exc.previous {
        let _ = write!(out, "\n  Caused by:{}", format_exception(prev, config));
    }

    out
}

fn format_stack_trace(trace: &StackTrace, config: &ConsoleConfig) -> String {
    match config.trace_format {
        TraceFormatKind::None => String::new(),
        TraceFormatKind::Basic => format_trace_basic(trace),
        TraceFormatKind::Full => format_trace_full(trace),
    }
}

fn format_trace_basic(trace: &StackTrace) -> String {
    let func = trace.function.as_deref().unwrap_or("");
    let cls = trace.class.as_deref().unwrap_or("");
    let call_type = trace.call_type.as_deref().unwrap_or("::");

    if func.is_empty() && !cls.is_empty() {
        return cls.to_string();
    }

    if func.is_empty() {
        if let (Some(ref file), Some(line)) = (&trace.file, trace.line) {
            return format!("{file}:{line}");
        } else if let Some(ref file) = trace.file {
            return file.clone();
        }
        return String::new();
    }

    if !cls.is_empty() {
        return format!("{cls}{call_type}{func}");
    }

    if let (Some(ref file), Some(line)) = (&trace.file, trace.line) {
        return format!("{file}:{line} {call_type} {func}");
    } else if let Some(ref file) = trace.file {
        return format!("{file} {call_type} {func}");
    }

    func.to_string()
}

fn format_trace_full(trace: &StackTrace) -> String {
    let mut out = String::new();

    if let Some(ref cls) = trace.class {
        let _ = write!(out, "{cls}");
    }

    if let Some(ref ct) = trace.call_type {
        let _ = write!(out, "{ct}");
    }

    if let Some(ref func) = trace.function {
        let _ = write!(out, "{func}");
    }

    if let Some(ref file) = trace.file {
        let _ = write!(out, " ({file})");
        if let Some(line) = trace.line {
            let _ = write!(out, ":{line}");
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SOURCE: &str = "127.0.0.1:12345";

    fn make_event(level: &str, message: &str) -> LogEvent {
        LogEvent {
            application_name: "TestApp".into(),
            timestamp: None,
            level: level.into(),
            message: message.into(),
            trace: None,
            traces: None,
            exception: None,
            source: Some(TEST_SOURCE.into()),
        }
    }

    fn make_event_with_exception() -> LogEvent {
        LogEvent {
            application_name: "App".into(),
            timestamp: None,
            level: "ERROR".into(),
            message: "something broke".into(),
            trace: None,
            traces: None,
            exception: Some(ExceptionDetails {
                name: "RuntimeException".into(),
                message: "null reference".into(),
                code: Some(500),
                file: Some("/src/main.php".into()),
                line: Some(42),
                trace: Some(vec![StackTrace {
                    file: Some("/src/main.php".into()),
                    line: Some(42),
                    function: Some("process".into()),
                    args: None,
                    class: Some("App\\Handler".into()),
                    call_type: Some("::".into()),
                }]),
                previous: None,
            }),
            source: Some(TEST_SOURCE.into()),
        }
    }

    #[test]
    fn test_format_info_level_no_color() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("INFO", "hello world");
        let (output, is_error) = format_event(&event, &config);
        assert!(!is_error);
        assert_eq!(output, "[TestApp] hello world");
    }

    #[test]
    fn test_format_error_level_is_error() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("ERROR", "oops");
        let (_, is_error) = format_event(&event, &config);
        assert!(is_error);
    }

    #[test]
    fn test_format_warning_level_is_error() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("WARNING", "caution");
        let (_, is_error) = format_event(&event, &config);
        assert!(is_error);
    }

    #[test]
    fn test_format_critical_level_is_error() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("CRITICAL", "fatal");
        let (_, is_error) = format_event(&event, &config);
        assert!(is_error);
    }

    #[test]
    fn test_format_dbg_level_not_error() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("DEBUG", "debug msg");
        let (_, is_error) = format_event(&event, &config);
        assert!(!is_error);
    }

    #[test]
    fn test_format_vrb_level_not_error() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event("VERBOSE", "verbose msg");
        let (_, is_error) = format_event(&event, &config);
        assert!(!is_error);
    }

    #[test]
    fn test_format_with_exception() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = make_event_with_exception();
        let (output, _) = format_event(&event, &config);
        assert!(output.contains("[App] something broke"));
        assert!(output.contains("RuntimeException"));
        assert!(output.contains("null reference"));
        assert!(output.contains("/src/main.php:42"));
    }

    #[test]
    fn test_format_with_stack_trace_always_show() {
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, true);
        let mut event = make_event("ERROR", "traced");
        event.traces = Some(vec![StackTrace {
            file: Some("/src/lib.php".into()),
            line: Some(10),
            function: Some("doStuff".into()),
            args: None,
            class: Some("Lib".into()),
            call_type: Some("::".into()),
        }]);
        let (output, _) = format_event(&event, &config);
        assert!(output.contains("Stack Trace:"));
        assert!(output.contains("Lib::doStuff"));
    }

    #[test]
    fn test_format_with_color() {
        let config = ConsoleConfig::new(true, TraceFormatKind::Basic, false);
        let event = make_event("ERROR", "colored");
        let (output, is_error) = format_event(&event, &config);
        assert!(is_error);
        assert!(output.contains("\x1b["));
        assert!(output.contains("TestApp"));
        assert!(output.contains("colored"));
        assert!(output.contains("\x1b[0m"));
    }

    #[test]
    fn test_format_trace_basic_class_only() {
        let trace = StackTrace {
            file: None,
            line: None,
            function: None,
            args: None,
            class: Some("MyClass".into()),
            call_type: None,
        };
        let result = format_trace_basic(&trace);
        assert_eq!(result, "MyClass");
    }

    #[test]
    fn test_format_trace_basic_file_and_line() {
        let trace = StackTrace {
            file: Some("/a.php".into()),
            line: Some(15),
            function: None,
            args: None,
            class: None,
            call_type: None,
        };
        let result = format_trace_basic(&trace);
        assert_eq!(result, "/a.php:15");
    }

    #[test]
    fn test_format_trace_basic_class_and_function() {
        let trace = StackTrace {
            file: None,
            line: None,
            function: Some("doIt".into()),
            args: None,
            class: Some("App\\Service".into()),
            call_type: Some("::".into()),
        };
        let result = format_trace_basic(&trace);
        assert_eq!(result, "App\\Service::doIt");
    }

    #[test]
    fn test_format_trace_basic_file_function_call_type() {
        let trace = StackTrace {
            file: Some("/b.php".into()),
            line: Some(20),
            function: Some("run".into()),
            args: None,
            class: None,
            call_type: Some("->".into()),
        };
        let result = format_trace_basic(&trace);
        assert_eq!(result, "/b.php:20 -> run");
    }

    #[test]
    fn test_format_trace_full() {
        let trace = StackTrace {
            file: Some("/c.php".into()),
            line: Some(30),
            function: Some("exec".into()),
            args: None,
            class: Some("Core".into()),
            call_type: Some("::".into()),
        };
        let result = format_trace_full(&trace);
        assert_eq!(result, "Core::exec (/c.php):30");
    }

    #[test]
    fn test_format_trace_full_no_file() {
        let trace = StackTrace {
            file: None,
            line: None,
            function: Some("anon".into()),
            args: None,
            class: Some("Closure".into()),
            call_type: None,
        };
        let result = format_trace_full(&trace);
        assert_eq!(result, "Closureanon");
    }

    #[test]
    fn test_format_trace_none() {
        let config = ConsoleConfig::new(false, TraceFormatKind::None, false);
        let event = make_event("INFO", "no trace");
        let (output, _) = format_event(&event, &config);
        assert_eq!(output, "[TestApp] no trace");
    }

    #[test]
    fn test_exception_with_chained_previous() {
        let inner = ExceptionDetails {
            name: "InnerError".into(),
            message: "inner msg".into(),
            code: None,
            file: Some("/inner.php".into()),
            line: Some(5),
            trace: None,
            previous: None,
        };
        let outer = ExceptionDetails {
            name: "OuterError".into(),
            message: "outer msg".into(),
            code: Some(1),
            file: Some("/outer.php".into()),
            line: Some(10),
            trace: None,
            previous: Some(Box::new(inner)),
        };
        let config = ConsoleConfig::new(false, TraceFormatKind::Basic, false);
        let event = LogEvent {
            application_name: "App".into(),
            timestamp: None,
            level: "ERROR".into(),
            message: "chained".into(),
            trace: None,
            traces: None,
            exception: Some(outer),
            source: None,
        };
        let (output, _) = format_event(&event, &config);
        assert!(output.contains("OuterError"));
        assert!(output.contains("outer msg"));
        assert!(output.contains("Caused by:"));
        assert!(output.contains("InnerError"));
        assert!(output.contains("inner msg"));
    }
}
