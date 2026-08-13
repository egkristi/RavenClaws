//! # Web Access Policy (domain-level)
//!
//! Category-based allowlist/blocklist policy for web-facing tools (`web_fetch`,
//! `web_search`, `browser`). This complements [`crate::policy::PolicyEngine`],
//! which is resource-level (shell/path/network allow-lists for *tool execution*);
//! this module operates at the *domain* level, classifying destinations into
//! named categories and enforcing per-category allow/block/permission rules.
//!
//! ## Architecture
//!
//! ```text
//! URL / domain
//!   │
//!   ▼
//! WebAccessPolicy::is_allowed()
//!   ├── policy disabled        → Allow("policy_disabled")
//!   ├── matches blocklist      → Deny("blocked by <category>")
//!   ├── category needs consent → Deny("permission required (<category>)")
//!   └── otherwise              → Allow("<category>")
//! ```
//!
//! The module also provides a [`RateLimiter`] for per-category rate limiting and
//! [`extract_domain`] to normalize a URL or search query into a bare domain.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ravenclaws::web_policy::{WebAccessPolicy, extract_domain};
//!
//! let policy = WebAccessPolicy::default();
//! let domain = extract_domain("https://docs.rs/ravenclaws");
//! let (allowed, reason) = policy.is_allowed(&domain);
//! assert!(allowed);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A named category of web destinations with its own allow/block lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCategory {
    /// Category name (e.g. "news", "code_repos", "social_media")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Domains classified into this category (substring match, case-insensitive)
    pub allowlist: Vec<String>,
    /// Domains explicitly blocked within this category (takes precedence)
    pub blocklist: Vec<String>,
    /// If true, access requires explicit user permission even if allowlisted
    pub require_permission: bool,
}

/// Domain-level web access policy.
///
/// # Stability
/// This struct is `#[non_exhaustive]` — new fields may be added in minor releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebAccessPolicy {
    /// Master switch — when false, all web access is allowed
    pub enabled: bool,
    /// Policy mode: "permissive" (allow unless blocked) or "strict" (deny unless allowlisted)
    pub mode: String,
    /// Per-category rate limit (requests per minute)
    pub rate_limit_per_minute: u32,
    /// Maximum concurrent fetches
    pub max_concurrent_fetches: u32,
    /// Maximum result size in bytes
    pub max_result_size_bytes: usize,
    /// Ordered category definitions
    pub categories: Vec<WebCategory>,
}

impl WebAccessPolicy {
    /// A policy that is disabled — all web access is allowed.
    ///
    /// This is the safe default for existing deployments; users opt into
    /// domain filtering by setting `enabled = true` in their configuration.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Whether the policy operates in strict mode (deny unless explicitly allowlisted).
    pub fn is_strict(&self) -> bool {
        self.mode.eq_ignore_ascii_case("strict")
    }

    /// Classify a domain into a category. Returns the first category whose
    /// allowlist or blocklist matches, or the final (uncategorized) category.
    pub fn classify(&self, domain: &str) -> &WebCategory {
        let domain_lower = domain.to_lowercase();
        for cat in &self.categories {
            if cat
                .blocklist
                .iter()
                .any(|d| domain_lower.contains(d.as_str()))
            {
                return cat;
            }
            if cat
                .allowlist
                .iter()
                .any(|d| domain_lower.contains(d.as_str()))
            {
                return cat;
            }
        }
        // Fall back to the last category (conventionally "uncategorized").
        self.categories
            .last()
            .expect("WebAccessPolicy categories must not be empty")
    }

    /// Check whether a domain is allowed by policy.
    ///
    /// Returns `(allowed, reason)` where `reason` is the category name on allow
    /// (or "policy_disabled") and a human-readable denial reason on deny.
    pub fn is_allowed(&self, domain: &str) -> (bool, String) {
        if !self.enabled {
            return (true, "policy_disabled".to_string());
        }
        let domain_lower = domain.to_lowercase();
        let cat = self.classify(domain);

        // Blocklist always wins.
        if cat
            .blocklist
            .iter()
            .any(|d| domain_lower.contains(d.as_str()))
        {
            return (false, format!("blocked by {} category", cat.name));
        }

        // Strict mode: only explicit allowlist entries pass.
        if self.is_strict()
            && !cat
                .allowlist
                .iter()
                .any(|d| domain_lower.contains(d.as_str()))
        {
            return (false, format!("not allowlisted (category: {})", cat.name));
        }

        // Permission-gated categories require consent.
        if cat.require_permission {
            return (
                false,
                format!("permission required (category: {})", cat.name),
            );
        }

        (true, cat.name.clone())
    }
}

impl Default for WebAccessPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "permissive".to_string(),
            rate_limit_per_minute: 30,
            max_concurrent_fetches: 5,
            max_result_size_bytes: 1_048_576,
            categories: default_categories(),
        }
    }
}

/// The default category set: news, search, code repositories, documentation,
/// social media (blocked by default), and an uncategorized catch-all.
pub fn default_categories() -> Vec<WebCategory> {
    vec![
        WebCategory {
            name: "news".to_string(),
            description: "News sites and media outlets".to_string(),
            allowlist: vec![
                "nrk.no".to_string(),
                "vg.no".to_string(),
                "bbc.com".to_string(),
                "cnn.com".to_string(),
                "reuters.com".to_string(),
                "apnews.com".to_string(),
                "theguardian.com".to_string(),
                "nytimes.com".to_string(),
                "dw.com".to_string(),
            ],
            blocklist: vec![],
            require_permission: false,
        },
        WebCategory {
            name: "search".to_string(),
            description: "Search engines".to_string(),
            allowlist: vec![
                "google.com".to_string(),
                "bing.com".to_string(),
                "duckduckgo.com".to_string(),
                "search.brave.com".to_string(),
                "kagi.com".to_string(),
            ],
            blocklist: vec![],
            require_permission: false,
        },
        WebCategory {
            name: "code_repos".to_string(),
            description: "Code repositories".to_string(),
            allowlist: vec![
                "github.com".to_string(),
                "gitlab.com".to_string(),
                "bitbucket.org".to_string(),
                "codeberg.org".to_string(),
                "gitea.com".to_string(),
            ],
            blocklist: vec![],
            require_permission: false,
        },
        WebCategory {
            name: "documentation".to_string(),
            description: "Technical docs and references".to_string(),
            allowlist: vec![
                "docs.rs".to_string(),
                "crates.io".to_string(),
                "pypi.org".to_string(),
                "npmjs.com".to_string(),
                "developer.mozilla.org".to_string(),
                "w3.org".to_string(),
                "stackoverflow.com".to_string(),
                "wikipedia.org".to_string(),
                "arxiv.org".to_string(),
                "kubernetes.io".to_string(),
                "helm.sh".to_string(),
            ],
            blocklist: vec![],
            require_permission: false,
        },
        WebCategory {
            name: "social_media".to_string(),
            description: "Social media platforms".to_string(),
            allowlist: vec![],
            blocklist: vec![
                "facebook.com".to_string(),
                "instagram.com".to_string(),
                "twitter.com".to_string(),
                "tiktok.com".to_string(),
                "snapchat.com".to_string(),
                "reddit.com".to_string(),
            ],
            require_permission: true,
        },
        WebCategory {
            name: "uncategorized".to_string(),
            description: "All other domains".to_string(),
            allowlist: vec![],
            blocklist: vec![],
            require_permission: true,
        },
    ]
}

/// Extract a bare domain from a URL or search query string.
///
/// Strips surrounding quotes and the URL scheme, then returns everything up to
/// the first `/`. For plain search queries (no scheme), returns the first token.
pub fn extract_domain(param_str: &str) -> String {
    let s = param_str.trim().trim_matches('"');
    if let Some(start) = s.find("://") {
        let after = &s[start + 3..];
        let end = after.find('/').unwrap_or(after.len());
        return after[..end].to_string();
    }
    let end = s.find('/').unwrap_or(s.len());
    s[..end].to_string()
}

/// A sliding-window (per-minute) rate limiter keyed by an arbitrary string
/// (typically a domain or category name).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: HashMap<String, (std::time::Instant, u32)>,
    max_per_minute: u32,
}

#[allow(dead_code)]
impl RateLimiter {
    /// Create a new rate limiter allowing `max_per_minute` requests per key.
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            buckets: HashMap::new(),
            max_per_minute,
        }
    }

    /// Check whether a request for `key` is within the rate limit.
    /// Returns `true` if the request is allowed.
    pub fn check(&mut self, key: &str) -> bool {
        let now = std::time::Instant::now();
        let entry = self.buckets.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0).as_secs() > 60 {
            *entry = (now, 1);
            return true;
        }
        if entry.1 >= self.max_per_minute {
            return false;
        }
        entry.1 += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_enabled_permissive() {
        let p = WebAccessPolicy::default();
        assert!(p.enabled);
        assert!(!p.is_strict());
        assert!(!p.categories.is_empty());
    }

    #[test]
    fn test_allowlisted_domain_passes() {
        let p = WebAccessPolicy::default();
        let (allowed, reason) = p.is_allowed("github.com");
        assert!(allowed);
        assert_eq!(reason, "code_repos");
    }

    #[test]
    fn test_blocklisted_domain_denied() {
        let p = WebAccessPolicy::default();
        let (allowed, reason) = p.is_allowed("facebook.com");
        assert!(!allowed);
        assert!(reason.contains("blocked"));
    }

    #[test]
    fn test_uncategorized_requires_permission() {
        let p = WebAccessPolicy::default();
        let (allowed, reason) = p.is_allowed("some-random-domain.example");
        assert!(!allowed);
        assert!(reason.contains("permission required"));
    }

    #[test]
    fn test_disabled_policy_allows_all() {
        let p = WebAccessPolicy {
            enabled: false,
            ..WebAccessPolicy::default()
        };
        let (allowed, reason) = p.is_allowed("facebook.com");
        assert!(allowed);
        assert_eq!(reason, "policy_disabled");
    }

    #[test]
    fn test_strict_mode_denies_unallowlisted() {
        let p = WebAccessPolicy {
            mode: "strict".to_string(),
            ..WebAccessPolicy::default()
        };
        assert!(p.is_strict());
        // github.com is allowlisted (code_repos) → allowed
        assert!(p.is_allowed("github.com").0);
        // arbitrary domain is not allowlisted → denied in strict mode
        let (allowed, reason) = p.is_allowed("not-in-any-list.example");
        assert!(!allowed);
        assert!(reason.contains("not allowlisted"));
    }

    #[test]
    fn test_classify_case_insensitive() {
        let p = WebAccessPolicy::default();
        assert_eq!(p.classify("GitHub.COM").name, "code_repos");
        assert_eq!(p.classify("github.com").name, "code_repos");
    }

    #[test]
    fn test_extract_domain_from_url() {
        assert_eq!(extract_domain("https://docs.rs/ravenclaws"), "docs.rs");
        assert_eq!(extract_domain("http://github.com/egkristi"), "github.com");
        assert_eq!(
            extract_domain("\"https://example.com/path\""),
            "example.com"
        );
    }

    #[test]
    fn test_extract_domain_from_query() {
        assert_eq!(extract_domain("rust documentation"), "rust documentation");
        assert_eq!(extract_domain("github.com"), "github.com");
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let mut rl = RateLimiter::new(2);
        assert!(rl.check("docs.rs"));
        assert!(rl.check("docs.rs"));
        assert!(!rl.check("docs.rs")); // third request within the minute is denied
    }

    #[test]
    fn test_rate_limiter_independent_keys() {
        let mut rl = RateLimiter::new(1);
        assert!(rl.check("a.example"));
        assert!(rl.check("b.example")); // different key, unaffected
        assert!(!rl.check("a.example"));
    }
}
