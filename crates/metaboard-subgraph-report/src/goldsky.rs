//! Minimal **read-only** client for Goldsky's subgraph admin API.
//!
//! Read-only by construction: the only request this module can issue is the
//! listing `GET`. There is deliberately no delete, pause or mutate path here.
//! Reaping is human-dispatched through the Goldsky CLI; this crate must not be
//! able to remove a deployed subgraph, including by mistake.

use crate::classify::{Kind, SubgraphEntry};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

/// Goldsky's API host.
pub const DEFAULT_API_HOST: &str = "https://api.goldsky.com";

/// The listing endpoint. This is the path the Goldsky CLI itself uses.
pub const SUBGRAPHS_PATH: &str = "/api/admin/subgraph/v1/subgraphs";

/// Bound the request so a slow or hung Goldsky endpoint cannot wedge a report.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum GoldskyError {
    #[error("goldsky request failed")]
    Http(#[from] reqwest::Error),
    #[error("goldsky returned HTTP {status}: {message}")]
    Api { status: u16, message: String },
    #[error("could not decode goldsky listing response")]
    Decode(#[source] serde_json::Error),
    #[error("no goldsky token supplied")]
    MissingToken,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(default)]
    data: Vec<SubgraphJson>,
}

/// Goldsky's error bodies carry `{"statusCode": .., "message": ".."}`.
#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    #[serde(default)]
    message: Option<String>,
}

/// One listing row as Goldsky returns it.
///
/// Only the fields classification needs are modelled, and every optional one
/// carries `#[serde(default)]` so an added or removed field upstream degrades
/// to a missing value rather than failing the whole report.
#[derive(Debug, Clone, Deserialize)]
pub struct SubgraphJson {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub tag: Option<TagJson>,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub graphql_endpoint: String,
    #[serde(default)]
    pub deployments: Vec<DeploymentJson>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagJson {
    #[serde(default)]
    pub target_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeploymentJson {
    /// Epoch millis. Goldsky's CLI reads this field as a JS `Date` argument.
    #[serde(default)]
    pub created_at: Option<i64>,
}

/// The value Goldsky uses for a paused subgraph's `status`.
const STATUS_PAUSED: &str = "Paused";

impl SubgraphJson {
    /// Reduce a listing row to the shape classification consumes.
    ///
    /// A row carrying a `tag` object is an alias row, whose `version` is the
    /// tag label rather than a deployed version. Keying on the presence of
    /// `tag` fails safe: an alias can never be mistaken for a deployment and
    /// so can never become a reaping candidate.
    pub fn into_entry(self) -> SubgraphEntry {
        let created_at_ms = self.deployments.iter().filter_map(|d| d.created_at).max();
        let kind = match self.tag {
            Some(tag) => Kind::TagAlias {
                target_version: tag.target_version,
            },
            None => Kind::Deployment,
        };
        SubgraphEntry {
            name: self.name,
            version: self.version,
            network: self.network,
            kind,
            paused: self.status.as_deref() == Some(STATUS_PAUSED),
            created_at_ms,
            graphql_endpoint: self.graphql_endpoint,
        }
    }
}

/// Pull the human-readable message out of a Goldsky error body, falling back
/// to the raw body when it is not the shape we expect.
fn error_message(body: &str) -> String {
    serde_json::from_str::<ApiErrorBody>(body)
        .ok()
        .and_then(|b| b.message)
        .unwrap_or_else(|| body.trim().to_string())
}

/// A read-only handle on Goldsky's subgraph listing.
pub struct GoldskyClient {
    http: reqwest::Client,
    api_host: String,
    token: String,
}

impl GoldskyClient {
    pub fn new(api_host: &str, token: &str) -> Result<Self, GoldskyError> {
        if token.is_empty() {
            return Err(GoldskyError::MissingToken);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            api_host: api_host.trim_end_matches('/').to_string(),
            token: token.to_string(),
        })
    }

    pub fn listing_url(&self) -> String {
        format!("{}{}", self.api_host, SUBGRAPHS_PATH)
    }

    /// `GET` the listing and reduce it to classification input.
    pub async fn list_subgraphs(&self) -> Result<Vec<SubgraphEntry>, GoldskyError> {
        let response = self
            .http
            .get(self.listing_url())
            .bearer_auth(&self.token)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(GoldskyError::Api {
                status: status.as_u16(),
                message: error_message(&body),
            });
        }

        let listing: ListResponse = serde_json::from_str(&body).map_err(GoldskyError::Decode)?;
        Ok(listing
            .data
            .into_iter()
            .map(SubgraphJson::into_entry)
            .collect())
    }
}

// httpmock runs a local tokio/hyper server, which has no wasm target, so this
// module and its dev-dependencies are gated to non-wasm together.
#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    fn parse(value: serde_json::Value) -> SubgraphJson {
        serde_json::from_value(value).expect("fixture should deserialize")
    }

    // ---------- row reduction ----------

    #[test]
    fn a_row_without_a_tag_object_is_a_deployment() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "0xfb84-1106a15",
            "network": "base",
            "status": "Active",
            "graphql_endpoint": "/api/public/p/subgraphs/metaboard-base/0xfb84-1106a15/gn",
            "deployments": [{ "created_at": 1_700_000_000_000i64 }]
        }))
        .into_entry();

        assert_eq!(entry.kind, Kind::Deployment);
        assert_eq!(entry.name_and_version(), "metaboard-base/0xfb84-1106a15");
        assert_eq!(entry.network, "base");
        assert!(!entry.paused);
        assert_eq!(entry.created_at_ms, Some(1_700_000_000_000));
    }

    #[test]
    fn a_row_carrying_a_tag_object_is_an_alias() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "latest",
            "tag": { "target_version": "0xfb84-1106a15" },
            "network": "base",
            "deployments": []
        }))
        .into_entry();

        assert_eq!(
            entry.kind,
            Kind::TagAlias {
                target_version: Some("0xfb84-1106a15".to_string())
            }
        );
        assert!(entry.is_tag_alias());
    }

    #[test]
    fn a_tag_object_with_no_target_is_a_dangling_alias() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "latest",
            "tag": {},
            "deployments": []
        }))
        .into_entry();

        assert_eq!(
            entry.kind,
            Kind::TagAlias {
                target_version: None
            }
        );
    }

    #[test]
    fn created_at_is_the_newest_of_several_deployment_records() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "v1",
            "deployments": [
                { "created_at": 100i64 },
                { "created_at": 900i64 },
                { "created_at": 500i64 }
            ]
        }))
        .into_entry();

        assert_eq!(entry.created_at_ms, Some(900));
    }

    #[test]
    fn no_deployment_records_means_an_unknown_creation_time() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "v1",
            "deployments": []
        }))
        .into_entry();

        assert_eq!(entry.created_at_ms, None);
    }

    #[test]
    fn deployment_records_without_a_timestamp_are_skipped() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "v1",
            "deployments": [{}, { "created_at": 42i64 }]
        }))
        .into_entry();

        assert_eq!(entry.created_at_ms, Some(42));
    }

    #[test]
    fn a_paused_status_is_carried_onto_the_entry() {
        let entry = parse(serde_json::json!({
            "name": "metaboard-base",
            "version": "v1",
            "status": "Paused",
            "deployments": []
        }))
        .into_entry();

        assert!(entry.paused);
    }

    #[test]
    fn only_the_exact_paused_status_counts_as_paused() {
        for status in ["Active", "paused", "PAUSED", ""] {
            let entry = parse(serde_json::json!({
                "name": "metaboard-base",
                "version": "v1",
                "status": status,
                "deployments": []
            }))
            .into_entry();
            assert!(!entry.paused, "status {status:?} should not read as paused");
        }
    }

    #[test]
    fn absent_optional_fields_degrade_rather_than_fail() {
        // Only name and version are required; everything else may be missing.
        let entry = parse(serde_json::json!({ "name": "n", "version": "v" })).into_entry();
        assert_eq!(entry.network, "");
        assert_eq!(entry.graphql_endpoint, "");
        assert_eq!(entry.created_at_ms, None);
        assert!(!entry.paused);
        assert_eq!(entry.kind, Kind::Deployment);
    }

    // ---------- error bodies ----------

    #[test]
    fn the_message_is_lifted_out_of_a_goldsky_error_body() {
        // This is the body the live endpoint returns for an unauthenticated
        // request, captured verbatim.
        let body = r#"{"statusCode":401,"message":"Make sure to run 'goldsky login' before running any commands requiring authorization."}"#;
        assert_eq!(
            error_message(body),
            "Make sure to run 'goldsky login' before running any commands requiring authorization."
        );
    }

    #[test]
    fn a_non_json_error_body_falls_back_to_the_raw_text() {
        assert_eq!(error_message("  502 Bad Gateway\n"), "502 Bad Gateway");
    }

    #[test]
    fn a_json_error_body_without_a_message_falls_back_to_the_raw_text() {
        assert_eq!(
            error_message(r#"{"statusCode":500}"#),
            r#"{"statusCode":500}"#
        );
    }

    // ---------- client construction ----------

    #[test]
    fn an_empty_token_is_refused_before_any_request_is_made() {
        assert!(matches!(
            GoldskyClient::new(DEFAULT_API_HOST, ""),
            Err(GoldskyError::MissingToken)
        ));
    }

    #[test]
    fn the_listing_url_is_the_host_plus_the_admin_path() {
        let client = GoldskyClient::new("https://api.goldsky.com", "t").unwrap();
        assert_eq!(
            client.listing_url(),
            "https://api.goldsky.com/api/admin/subgraph/v1/subgraphs"
        );
    }

    #[test]
    fn a_trailing_slash_on_the_host_does_not_double_up() {
        let client = GoldskyClient::new("https://api.goldsky.com/", "t").unwrap();
        assert_eq!(
            client.listing_url(),
            "https://api.goldsky.com/api/admin/subgraph/v1/subgraphs"
        );
    }

    // ---------- the HTTP path ----------

    #[tokio::test]
    async fn a_listing_is_fetched_and_reduced_to_entries() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(200).json_body(serde_json::json!({ "data": [
                    {
                        "name": "metaboard-base",
                        "version": "0xfb84-1106a15",
                        "network": "base",
                        "status": "Active",
                        "graphql_endpoint": "/gn",
                        "deployments": [{ "created_at": 1_700_000_000_000i64 }]
                    },
                    {
                        "name": "metaboard-base",
                        "version": "latest",
                        "tag": { "target_version": "0xfb84-1106a15" },
                        "network": "base",
                        "deployments": []
                    }
                ]}));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        let entries = client.list_subgraphs().await.unwrap();

        mock.assert_async().await;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, Kind::Deployment);
        assert!(entries[1].is_tag_alias());
    }

    #[tokio::test]
    async fn the_token_is_sent_as_a_bearer_authorization_header() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path(SUBGRAPHS_PATH)
                    .header("authorization", "Bearer super-secret");
                then.status(200)
                    .json_body(serde_json::json!({ "data": [] }));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "super-secret").unwrap();
        client.list_subgraphs().await.unwrap();

        // The mock only matches when the header is present and exact.
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn an_empty_listing_is_not_an_error() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(200)
                    .json_body(serde_json::json!({ "data": [] }));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        assert!(client.list_subgraphs().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_response_with_no_data_key_reads_as_an_empty_listing() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(200).json_body(serde_json::json!({}));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        assert!(client.list_subgraphs().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unauthorized_response_surfaces_the_status_and_message() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(401).json_body(serde_json::json!({
                    "statusCode": 401,
                    "message": "Make sure to run 'goldsky login' before running any commands requiring authorization."
                }));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "bad").unwrap();
        let error = client.list_subgraphs().await.unwrap_err();

        match error {
            GoldskyError::Api {
                status,
                ref message,
            } => {
                assert_eq!(status, 401);
                assert!(message.contains("goldsky login"));
            }
            other => panic!("expected an Api error, got {other:?}"),
        }
        assert!(error.to_string().contains("401"));
    }

    #[tokio::test]
    async fn a_server_error_with_a_non_json_body_still_surfaces() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(500).body("upstream exploded");
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        match client.list_subgraphs().await.unwrap_err() {
            GoldskyError::Api { status, message } => {
                assert_eq!(status, 500);
                assert_eq!(message, "upstream exploded");
            }
            other => panic!("expected an Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_malformed_success_body_is_a_decode_error_not_a_silent_empty() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(200).body("not json at all");
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        assert!(matches!(
            client.list_subgraphs().await.unwrap_err(),
            GoldskyError::Decode(_)
        ));
    }

    #[tokio::test]
    async fn a_row_missing_its_required_fields_is_a_decode_error() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(GET).path(SUBGRAPHS_PATH);
                then.status(200)
                    .json_body(serde_json::json!({ "data": [{ "name": "n" }] }));
            })
            .await;

        let client = GoldskyClient::new(&server.base_url(), "token").unwrap();
        assert!(matches!(
            client.list_subgraphs().await.unwrap_err(),
            GoldskyError::Decode(_)
        ));
    }
}
