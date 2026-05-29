use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub application_name: String,
    #[serde(default)]
    pub timestamp: Option<serde_json::Value>,
    #[serde(default)]
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub trace: Option<String>,
    #[serde(default, alias = "traces", rename = "stack_trace")]
    pub traces: Option<Vec<StackTrace>>,
    #[serde(default)]
    pub exception: Option<ExceptionDetails>,
    #[serde(default, skip_deserializing)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTrace {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<i64>,
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub call_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionDetails {
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub code: Option<i64>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<i64>,
    #[serde(default)]
    pub trace: Option<Vec<StackTrace>>,
    #[serde(default)]
    pub previous: Option<Box<Self>>,
}

impl LogEvent {
    pub fn level_value(&self) -> &str {
        match self.level.to_uppercase().as_str() {
            "DBG" | "DEBUG" => "DBG",
            "VRB" | "VERBOSE" => "VRB",
            "WRN" | "WARNING" => "WRN",
            "ERR" | "ERROR" => "ERR",
            "CRT" | "CRITICAL" => "CRT",
            _ => "INFO",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_value_dbg() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "DEBUG".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "DBG");
    }

    #[test]
    fn test_level_value_vrb() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "VERBOSE".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "VRB");
    }

    #[test]
    fn test_level_value_info() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "INFO".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "INFO");
    }

    #[test]
    fn test_level_value_wrn() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "WARNING".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "WRN");
    }

    #[test]
    fn test_level_value_err() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "ERROR".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "ERR");
    }

    #[test]
    fn test_level_value_crt() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "CRITICAL".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "CRT");
    }

    #[test]
    fn test_level_value_unknown_defaults_to_info() {
        let event = LogEvent {
            application_name: "test".into(),
            timestamp: None,
            level: "UNKNOWN".into(),
            message: "msg".into(),
            trace: None,
            traces: None,
            exception: None,
            source: None,
        };
        assert_eq!(event.level_value(), "INFO");
    }

    #[test]
    fn test_log_event_deserialize_basic() {
        let json = r#"{"application_name":"MyApp","level":"ERROR","message":"test error"}"#;
        let event: LogEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.application_name, "MyApp");
        assert_eq!(event.level, "ERROR");
        assert_eq!(event.message, "test error");
        assert!(event.timestamp.is_none());
        assert!(event.trace.is_none());
        assert!(event.traces.is_none());
        assert!(event.exception.is_none());
        assert!(event.source.is_none());
    }

    #[test]
    fn test_log_event_deserialize_with_traces() {
        let json = r#"{
            "application_name": "MyApp",
            "level": "ERROR",
            "message": "failed",
            "traces": [
                {"file": "/a.php", "line": 10, "function": "foo", "class": "A", "call_type": "::"}
            ]
        }"#;
        let event: LogEvent = serde_json::from_str(json).unwrap();
        let traces = event.traces.unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].file.as_deref(), Some("/a.php"));
        assert_eq!(traces[0].line, Some(10));
        assert_eq!(traces[0].function.as_deref(), Some("foo"));
        assert_eq!(traces[0].class.as_deref(), Some("A"));
    }

    #[test]
    fn test_log_event_deserialize_stack_trace_alias() {
        let json = r#"{
            "application_name": "App",
            "level": "ERR",
            "message": "oops",
            "stack_trace": [
                {"file": "/b.php", "line": 5, "function": "bar"}
            ]
        }"#;
        let event: LogEvent = serde_json::from_str(json).unwrap();
        let traces = event.traces.unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].function.as_deref(), Some("bar"));
    }

    #[test]
    fn test_log_event_deserialize_with_exception() {
        let json = r#"{
            "application_name": "App",
            "level": "CRITICAL",
            "message": "critical failure",
            "exception": {
                "name": "RuntimeException",
                "message": "null ref",
                "code": 500,
                "file": "/c.php",
                "line": 42
            }
        }"#;
        let event: LogEvent = serde_json::from_str(json).unwrap();
        let exc = event.exception.unwrap();
        assert_eq!(exc.name, "RuntimeException");
        assert_eq!(exc.message, "null ref");
        assert_eq!(exc.code, Some(500));
        assert_eq!(exc.file.as_deref(), Some("/c.php"));
        assert_eq!(exc.line, Some(42));
        assert!(exc.trace.is_none());
        assert!(exc.previous.is_none());
    }

    #[test]
    fn test_log_event_deserialize_with_chained_exception() {
        let json = r#"{
            "application_name": "App",
            "level": "ERR",
            "message": "chain",
            "exception": {
                "name": "OuterException",
                "message": "outer",
                "previous": {
                    "name": "InnerException",
                    "message": "inner"
                }
            }
        }"#;
        let event: LogEvent = serde_json::from_str(json).unwrap();
        let exc = event.exception.unwrap();
        assert_eq!(exc.name, "OuterException");
        let prev = exc.previous.unwrap();
        assert_eq!(prev.name, "InnerException");
        assert!(prev.previous.is_none());
    }
}
