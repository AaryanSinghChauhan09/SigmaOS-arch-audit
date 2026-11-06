// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-arch-audit — package security auditing, arch-audit-inspired.
//!
//! Models a local security-advisory database and an audit pass over
//! installed packages: each advisory names an affected package and severity.
//! `audit` returns all open advisories for a package set, sorted by
//! severity (Critical first). Fail-fast on malformed advisories.

use std::cmp::Ordering;
use std::fmt;

/// Errors building advisories or auditing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// Advisory ID or package name is empty.
    EmptyField(&'static str),
    /// Unknown severity string.
    BadSeverity(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "empty field: {field}"),
            Self::BadSeverity(s) => write!(f, "unknown severity: {s}"),
        }
    }
}

impl std::error::Error for AuditError {}

/// Advisory severity, ordered Critical > High > Medium > Low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Remote code execution / privilege escalation.
    Critical,
    /// Significant risk, exploitation likely.
    High,
    /// Limited impact or difficult exploitation.
    Medium,
    /// Minor / informational.
    Low,
}

impl Severity {
    fn rank(self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::High => 1,
            Self::Medium => 2,
            Self::Low => 3,
        }
    }

    /// Parse severity names used by the tracker.
    pub fn parse(s: &str) -> Result<Self, AuditError> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            other => Err(AuditError::BadSeverity(other.to_string())),
        }
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}
impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A security advisory affecting one package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advisory {
    /// Unique advisory identifier (e.g. `CVE-2026-1234`).
    pub id: String,
    /// Affected package name.
    pub package: String,
    /// Severity rating.
    pub severity: Severity,
    /// Fixed-in version, if a fix exists.
    pub fixed_version: Option<String>,
}

impl Advisory {
    /// Create an advisory; fails fast on empty id/package.
    pub fn new(
        id: impl Into<String>,
        package: impl Into<String>,
        severity: Severity,
    ) -> Result<Self, AuditError> {
        let (id, package) = (id.into(), package.into());
        if id.trim().is_empty() {
            return Err(AuditError::EmptyField("id"));
        }
        if package.trim().is_empty() {
            return Err(AuditError::EmptyField("package"));
        }
        Ok(Self {
            id,
            package,
            severity,
            fixed_version: None,
        })
    }
}

/// Advisory database.
#[derive(Debug, Default)]
pub struct AuditDb {
    advisories: Vec<Advisory>,
}

impl AuditDb {
    /// Empty database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an advisory.
    pub fn add(&mut self, advisory: Advisory) {
        self.advisories.push(advisory);
    }

    /// Audit a set of installed package names; returns matching advisories
    /// sorted by severity (most severe first), then by id for determinism.
    pub fn audit(&self, installed: &[&str]) -> Vec<&Advisory> {
        let mut hits: Vec<&Advisory> = self
            .advisories
            .iter()
            .filter(|a| installed.contains(&a.package.as_str()))
            .collect();
        hits.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .then_with(|| a.id.cmp(&b.id))
        });
        hits
    }

    /// Highest severity among advisories for one package, if any.
    pub fn worst_for(&self, package: &str) -> Option<Severity> {
        self.advisories
            .iter()
            .filter(|a| a.package == package)
            .map(|a| a.severity)
            .min()
    }

    /// Total advisory count.
    pub fn len(&self) -> usize {
        self.advisories.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.advisories.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> AuditDb {
        let mut db = AuditDb::new();
        db.add(Advisory::new("CVE-1", "linux", Severity::High).unwrap());
        db.add(Advisory::new("CVE-2", "linux", Severity::Critical).unwrap());
        db.add(Advisory::new("CVE-3", "bash", Severity::Low).unwrap());
        db.add(Advisory::new("CVE-4", "openssl", Severity::Medium).unwrap());
        db
    }

    #[test]
    fn audit_returns_matching_sorted_by_severity() {
        let d = db();
        let hits = d.audit(&["linux"]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].severity, Severity::Critical);
        assert_eq!(hits[1].severity, Severity::High);
    }

    #[test]
    fn audit_multiple_packages() {
        let d = db();
        let hits = d.audit(&["bash", "openssl"]);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].package, "openssl"); // Medium < Low
        assert_eq!(hits[1].package, "bash");
    }

    #[test]
    fn audit_no_match_empty() {
        let d = db();
        assert!(d.audit(&["vim"]).is_empty());
    }

    #[test]
    fn worst_for_returns_min_severity() {
        assert_eq!(db().worst_for("linux"), Some(Severity::Critical));
        assert_eq!(db().worst_for("vim"), None);
    }

    #[test]
    fn empty_fields_fail() {
        assert!(Advisory::new("", "linux", Severity::High).is_err());
        assert!(Advisory::new("CVE", " ", Severity::High).is_err());
    }

    #[test]
    fn severity_parse() {
        assert_eq!(Severity::parse("CRITICAL").unwrap(), Severity::Critical);
        assert!(Severity::parse("bogus").is_err());
    }
}
