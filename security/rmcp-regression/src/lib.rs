// SPDX-License-Identifier: MPL-2.0
//! Regression tests for the source maintenance backport. Chrono is an optional
//! test oracle, never a dependency of the shipping Glyphibe feature selection.
#[cfg(test)]
#[path = "../../../vendor/rmcp/src/clock.rs"]
mod clock;

#[cfg(test)]
mod tests {
    use rmcp::model::{Annotations, ClientJsonRpcMessage, ServerJsonRpcMessage};
    use serde_json::{Value, json};
    use std::time::Duration;

    #[test]
    fn annotation_wire_contract_is_unchanged() {
        for timestamp in [
            "2025-01-12",
            "not-a-date",
            "2025-01-12T15:00:58Z",
            "2025-01-12T23:00:58+08:00",
        ] {
            let original =
                json!({"lastModified": timestamp, "priority": 0.5, "audience": ["user"]});
            let value: Annotations = serde_json::from_value(original.clone()).unwrap();
            assert_eq!(serde_json::to_value(value).unwrap(), original);
        }
        for original in [json!({}), json!({"lastModified": null})] {
            let value: Annotations = serde_json::from_value(original).unwrap();
            assert_eq!(value.last_modified, None);
            assert_eq!(serde_json::to_value(value).unwrap(), json!({}));
        }
    }

    #[test]
    fn live_clock_is_rfc3339_and_keeps_explicit_utc_offset() {
        let before = time::OffsetDateTime::now_utc();
        let generated = Annotations::default()
            .with_timestamp_now()
            .last_modified
            .unwrap();
        let parsed =
            time::OffsetDateTime::parse(&generated, &time::format_description::well_known::Rfc3339)
                .unwrap();
        assert!(generated.ends_with("+00:00"));
        assert!(parsed >= before && parsed <= time::OffsetDateTime::now_utc());
        let millis = super::clock::current_timestamp_millis();
        assert!(
            (i128::from(millis)
                - time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
                .abs()
                < 2_000
        );
        assert!(!super::clock::current_timestamp().is_empty());
        assert!(!rmcp::task_manager::current_timestamp().is_empty());
    }

    #[test]
    fn public_timeout_error_keeps_original_display() {
        assert_eq!(
            rmcp::service::ServiceError::Timeout {
                timeout: Duration::new(42, 120_000_000)
            }
            .to_string(),
            "request timeout after PT42.12S"
        );
        assert_eq!(
            rmcp::service::ServiceError::Timeout {
                timeout: Duration::from_secs(u64::MAX)
            }
            .to_string(),
            "request timeout after P0D"
        );
    }

    #[test]
    fn public_schemas_match_unmodified_upstream_fixtures() {
        let settings = schemars::generate::SchemaSettings::draft07();
        let client = settings
            .clone()
            .into_generator()
            .into_root_schema_for::<ClientJsonRpcMessage>();
        let server = settings
            .into_generator()
            .into_root_schema_for::<ServerJsonRpcMessage>();
        let client_expected: Value = serde_json::from_str(include_str!(
            "../../../vendor/rmcp/tests/test_message_schema/client_json_rpc_message_schema.json"
        ))
        .unwrap();
        let server_expected: Value = serde_json::from_str(include_str!(
            "../../../vendor/rmcp/tests/test_message_schema/server_json_rpc_message_schema.json"
        ))
        .unwrap();
        assert_eq!(serde_json::to_value(client).unwrap(), client_expected);
        assert_eq!(serde_json::to_value(server).unwrap(), server_expected);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn legacy_typed_constructors_remain_compatible() {
        let timestamp = chrono::DateTime::from_timestamp(1_700_000_000, 120_000_000).unwrap();
        let annotations = Annotations::for_resource(0.5, timestamp);
        assert_eq!(
            annotations.last_modified.as_deref(),
            Some("2023-11-14T22:13:20.120+00:00")
        );
        assert_eq!(
            Annotations::default()
                .with_timestamp(timestamp)
                .last_modified,
            annotations.last_modified
        );
        assert!(std::panic::catch_unwind(|| Annotations::for_resource(1.1, timestamp)).is_err());
    }
}
