//! Enumerate the `metaboard` subgraphs deployed on Goldsky and report which
//! deployed versions have been superseded.
//!
//! # Why supersession and not usage
//!
//! rain.metadata.deploy#3 asks for per-deployment usage stats (queries,
//! bandwidth, last-query timestamp) and a reaping list derived from them.
//! Goldsky does not expose that. Its subgraph admin API — the API its own CLI
//! drives — has a listing endpoint, per-version deployment records, tags,
//! logs, pause/start and webhooks, and no usage or metrics endpoint of any
//! kind. There is no field on a listing row carrying a query count, a
//! bandwidth figure or a last-query time.
//!
//! So this crate reports the signal that does exist, and which is the one the
//! issue's own trigger describes: the deploy is idempotent **by name**, so it
//! skips a version already deployed and never removes the version it replaced.
//! Old address-plus-commit slots stay live indefinitely. A version that a
//! newer version of the same name has replaced, that no tag resolves to, and
//! that nobody pinned, is *superseded* — reported here as a reaping candidate
//! for a human to confirm and act on.
//!
//! Superseded is not the same claim as unused, and this crate never makes the
//! second one. Every report says so in its own output.
//!
//! # This crate cannot delete anything
//!
//! [`goldsky::GoldskyClient`] can issue exactly one request: the listing
//! `GET`. No delete, pause or mutate path exists in this crate, by
//! construction rather than by policy. The output is a list of `name/version`
//! identifiers and the reason each was selected; turning that into a deletion
//! is a human dispatching the Goldsky CLI.

pub mod classify;
pub mod goldsky;
pub mod report;
