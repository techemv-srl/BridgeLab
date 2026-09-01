//! Proprietary BridgeLab feature implementations.
//!
//! Everything under this directory is licensed under the Business Source
//! License 1.1 — see the LICENSE file in this directory. It is compiled
//! only when the `pro` cargo feature is enabled (the default for official
//! TECHEMV builds); Community-only builds (`--no-default-features`)
//! exclude it entirely and the MIT command shims answer instead.

pub mod soap;
