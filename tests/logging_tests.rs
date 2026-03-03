//! Tests for the logging module: log path, label safety, and file appends.

#[cfg(test)]
mod logging_tests {
    use ttyms::logging::{init_logging, is_safe_event_label, log_event, log_file_path};

    #[test]
    fn log_file_path_uses_ttyms_log_file() {
        let path = log_file_path().expect("log path should resolve");
        assert_eq!(path.file_name().and_then(|f| f.to_str()), Some("ttyms.log"));
        assert!(
            path.to_string_lossy().to_lowercase().contains("ttyms"),
            "path should include app directory: {}",
            path.display()
        );
    }

    #[test]
    fn safe_event_labels_allow_expected_format() {
        assert!(is_safe_event_label("app.start"));
        assert!(is_safe_event_label("auth.device_code.success"));
        assert!(is_safe_event_label("failure.config_load"));
        assert!(is_safe_event_label("event-1_2.3"));
    }

    #[test]
    fn safe_event_labels_reject_pii_like_patterns() {
        assert!(!is_safe_event_label(""));
        assert!(!is_safe_event_label("user@example.com"));
        assert!(!is_safe_event_label("https://example.com"));
        assert!(!is_safe_event_label("contains spaces"));
        assert!(!is_safe_event_label("name:john"));
    }

    #[test]
    fn init_logging_and_event_append() {
        init_logging().expect("logger initialization should succeed");
        log_event("test.healthcheck").expect("event append should succeed");

        let path = log_file_path().expect("log path should resolve");
        let content = std::fs::read_to_string(&path).expect("log file should be readable");
        assert!(content.contains("test.healthcheck"));
    }
}
