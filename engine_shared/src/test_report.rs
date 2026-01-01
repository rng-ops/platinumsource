//! Test report generation with Steamworks-style HTML output.
//!
//! This module provides infrastructure for generating beautiful HTML test reports
//! that match the Steamworks documentation aesthetic. Reports include:
//! - Test results by category
//! - Coverage metrics
//! - Valve documentation references
//! - CI/CD integration metadata

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Test result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Pending,
}

impl TestStatus {
    pub fn css_class(&self) -> &'static str {
        match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::Pending => "pending",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TestStatus::Passed => "✓",
            TestStatus::Failed => "✗",
            TestStatus::Skipped => "○",
            TestStatus::Pending => "◐",
        }
    }
}

/// Priority level for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl TestPriority {
    pub fn css_class(&self) -> &'static str {
        match self {
            TestPriority::Critical => "priority-critical",
            TestPriority::High => "priority-high",
            TestPriority::Medium => "priority-medium",
            TestPriority::Low => "priority-low",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TestPriority::Critical => "P0",
            TestPriority::High => "P1",
            TestPriority::Medium => "P2",
            TestPriority::Low => "P3",
        }
    }
}

/// A single test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test ID (e.g., "AUTH-001").
    pub id: String,
    /// Test name.
    pub name: String,
    /// Test description.
    pub description: String,
    /// Test category.
    pub category: String,
    /// Test status.
    pub status: TestStatus,
    /// Test priority.
    pub priority: TestPriority,
    /// Execution duration.
    pub duration: Duration,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Stack trace if failed.
    pub stack_trace: Option<String>,
    /// Valve documentation reference.
    pub doc_reference: Option<String>,
    /// Associated file/module.
    pub source_file: Option<String>,
    /// Line number.
    pub line_number: Option<u32>,
}

impl TestResult {
    pub fn new(id: &str, name: &str, category: &str) -> Self {
        TestResult {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            category: category.to_string(),
            status: TestStatus::Pending,
            priority: TestPriority::Medium,
            duration: Duration::ZERO,
            error_message: None,
            stack_trace: None,
            doc_reference: None,
            source_file: None,
            line_number: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_priority(mut self, priority: TestPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_doc_reference(mut self, url: &str) -> Self {
        self.doc_reference = Some(url.to_string());
        self
    }

    pub fn with_source(mut self, file: &str, line: u32) -> Self {
        self.source_file = Some(file.to_string());
        self.line_number = Some(line);
        self
    }

    pub fn pass(mut self, duration: Duration) -> Self {
        self.status = TestStatus::Passed;
        self.duration = duration;
        self
    }

    pub fn fail(mut self, duration: Duration, error: &str) -> Self {
        self.status = TestStatus::Failed;
        self.duration = duration;
        self.error_message = Some(error.to_string());
        self
    }

    pub fn skip(mut self, reason: &str) -> Self {
        self.status = TestStatus::Skipped;
        self.error_message = Some(reason.to_string());
        self
    }
}

/// Category summary statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryStats {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub pending: u32,
    pub total_duration: Duration,
}

impl CategoryStats {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }

    pub fn add_result(&mut self, result: &TestResult) {
        self.total += 1;
        self.total_duration += result.duration;
        match result.status {
            TestStatus::Passed => self.passed += 1,
            TestStatus::Failed => self.failed += 1,
            TestStatus::Skipped => self.skipped += 1,
            TestStatus::Pending => self.pending += 1,
        }
    }
}

/// Full test report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    /// Report title.
    pub title: String,
    /// Report subtitle/description.
    pub subtitle: String,
    /// Timestamp when report was generated.
    pub timestamp: u64,
    /// Git commit hash (if available).
    pub git_commit: Option<String>,
    /// Git branch (if available).
    pub git_branch: Option<String>,
    /// CI/CD build number (if available).
    pub build_number: Option<String>,
    /// All test results.
    pub results: Vec<TestResult>,
    /// Coverage percentage (if available).
    pub coverage_percent: Option<f64>,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl TestReport {
    pub fn new(title: &str, subtitle: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        TestReport {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            timestamp,
            git_commit: None,
            git_branch: None,
            build_number: None,
            results: Vec::new(),
            coverage_percent: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a test result.
    pub fn add_result(&mut self, result: TestResult) {
        self.results.push(result);
    }

    /// Get overall stats.
    pub fn overall_stats(&self) -> CategoryStats {
        let mut stats = CategoryStats::default();
        for result in &self.results {
            stats.add_result(result);
        }
        stats
    }

    /// Get stats by category.
    pub fn stats_by_category(&self) -> HashMap<String, CategoryStats> {
        let mut map: HashMap<String, CategoryStats> = HashMap::new();
        for result in &self.results {
            map.entry(result.category.clone())
                .or_default()
                .add_result(result);
        }
        map
    }

    /// Get results by category.
    pub fn results_by_category(&self) -> HashMap<String, Vec<&TestResult>> {
        let mut map: HashMap<String, Vec<&TestResult>> = HashMap::new();
        for result in &self.results {
            map.entry(result.category.clone()).or_default().push(result);
        }
        map
    }

    /// Check if all tests passed.
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.status == TestStatus::Passed)
    }

    /// Generate HTML report.
    pub fn to_html(&self) -> String {
        let stats = self.overall_stats();
        let by_category = self.stats_by_category();
        let results_by_cat = self.results_by_category();

        let mut html = String::new();
        html.push_str(&self.html_header());
        html.push_str(&self.html_summary(&stats));
        html.push_str(&self.html_categories(&by_category));
        html.push_str(&self.html_results(&results_by_cat));
        html.push_str(&self.html_footer());
        html
    }

    fn html_header(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Test Report</title>
    <style>
        :root {{
            --steam-dark: #1b2838;
            --steam-darker: #171a21;
            --steam-blue: #66c0f4;
            --steam-light-blue: #b8d4e3;
            --steam-green: #5ba32b;
            --steam-red: #c74545;
            --steam-yellow: #ffc82c;
            --steam-gray: #8f98a0;
            --steam-light: #c7d5e0;
        }}
        
        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}
        
        body {{
            font-family: 'Motiva Sans', Arial, Helvetica, sans-serif;
            background: linear-gradient(to bottom, var(--steam-dark), var(--steam-darker));
            color: var(--steam-light);
            min-height: 100vh;
            line-height: 1.6;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }}
        
        header {{
            background: var(--steam-darker);
            border-bottom: 1px solid rgba(255,255,255,0.1);
            padding: 20px 0;
            margin-bottom: 30px;
        }}
        
        header h1 {{
            color: var(--steam-blue);
            font-size: 2.5em;
            font-weight: 300;
        }}
        
        header .subtitle {{
            color: var(--steam-gray);
            font-size: 1.1em;
            margin-top: 5px;
        }}
        
        header .meta {{
            margin-top: 15px;
            font-size: 0.9em;
            color: var(--steam-gray);
        }}
        
        header .meta span {{
            margin-right: 20px;
        }}
        
        .summary-cards {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        
        .card {{
            background: rgba(0,0,0,0.3);
            border-radius: 4px;
            padding: 20px;
            border: 1px solid rgba(255,255,255,0.1);
        }}
        
        .card h3 {{
            color: var(--steam-gray);
            font-size: 0.9em;
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-bottom: 10px;
        }}
        
        .card .value {{
            font-size: 2.5em;
            font-weight: 300;
        }}
        
        .card .value.passed {{ color: var(--steam-green); }}
        .card .value.failed {{ color: var(--steam-red); }}
        .card .value.skipped {{ color: var(--steam-yellow); }}
        .card .value.total {{ color: var(--steam-blue); }}
        
        .progress-bar {{
            height: 8px;
            background: rgba(0,0,0,0.3);
            border-radius: 4px;
            overflow: hidden;
            margin-top: 10px;
        }}
        
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, var(--steam-green), var(--steam-blue));
            transition: width 0.3s ease;
        }}
        
        .category-section {{
            margin-bottom: 40px;
        }}
        
        .category-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 15px 20px;
            background: rgba(0,0,0,0.2);
            border-radius: 4px 4px 0 0;
            border: 1px solid rgba(255,255,255,0.1);
            border-bottom: none;
            cursor: pointer;
        }}
        
        .category-header:hover {{
            background: rgba(255,255,255,0.05);
        }}
        
        .category-header h2 {{
            color: var(--steam-blue);
            font-size: 1.3em;
            font-weight: 400;
        }}
        
        .category-stats {{
            display: flex;
            gap: 15px;
            font-size: 0.9em;
        }}
        
        .category-stats .stat {{
            padding: 4px 12px;
            border-radius: 3px;
            background: rgba(0,0,0,0.3);
        }}
        
        .stat.passed {{ color: var(--steam-green); }}
        .stat.failed {{ color: var(--steam-red); }}
        .stat.skipped {{ color: var(--steam-yellow); }}
        
        .test-list {{
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 0 0 4px 4px;
        }}
        
        .test-item {{
            display: grid;
            grid-template-columns: auto 60px 1fr auto auto;
            gap: 15px;
            align-items: center;
            padding: 12px 20px;
            border-bottom: 1px solid rgba(255,255,255,0.05);
        }}
        
        .test-item:last-child {{
            border-bottom: none;
        }}
        
        .test-item:hover {{
            background: rgba(255,255,255,0.02);
        }}
        
        .test-status {{
            width: 24px;
            height: 24px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 14px;
        }}
        
        .test-status.passed {{ background: var(--steam-green); color: white; }}
        .test-status.failed {{ background: var(--steam-red); color: white; }}
        .test-status.skipped {{ background: var(--steam-yellow); color: black; }}
        .test-status.pending {{ background: var(--steam-gray); color: white; }}
        
        .test-id {{
            font-family: monospace;
            color: var(--steam-blue);
            font-size: 0.85em;
        }}
        
        .test-name {{
            font-weight: 500;
        }}
        
        .test-description {{
            font-size: 0.85em;
            color: var(--steam-gray);
            margin-top: 3px;
        }}
        
        .test-priority {{
            font-size: 0.75em;
            padding: 3px 8px;
            border-radius: 3px;
            font-weight: bold;
        }}
        
        .priority-critical {{ background: var(--steam-red); color: white; }}
        .priority-high {{ background: #ff6b35; color: white; }}
        .priority-medium {{ background: var(--steam-yellow); color: black; }}
        .priority-low {{ background: var(--steam-gray); color: white; }}
        
        .test-duration {{
            font-size: 0.85em;
            color: var(--steam-gray);
            font-family: monospace;
        }}
        
        .test-error {{
            grid-column: 1 / -1;
            background: rgba(199, 69, 69, 0.1);
            border-left: 3px solid var(--steam-red);
            padding: 10px 15px;
            margin-top: 10px;
            font-family: monospace;
            font-size: 0.85em;
            white-space: pre-wrap;
        }}
        
        .doc-link {{
            color: var(--steam-blue);
            text-decoration: none;
            font-size: 0.85em;
        }}
        
        .doc-link:hover {{
            text-decoration: underline;
        }}
        
        footer {{
            margin-top: 50px;
            padding: 20px;
            text-align: center;
            color: var(--steam-gray);
            font-size: 0.85em;
            border-top: 1px solid rgba(255,255,255,0.1);
        }}
        
        @media (max-width: 768px) {{
            .test-item {{
                grid-template-columns: auto 1fr;
            }}
            .test-id, .test-priority, .test-duration {{
                grid-column: 2;
            }}
        }}
    </style>
</head>
<body>
    <header>
        <div class="container">
            <h1>{}</h1>
            <div class="subtitle">{}</div>
            <div class="meta">
                <span>Generated: {}</span>
                {}
                {}
                {}
            </div>
        </div>
    </header>
    <div class="container">
"#,
            self.title,
            self.title,
            self.subtitle,
            chrono_format(self.timestamp),
            self.git_commit
                .as_ref()
                .map(|c| format!("<span>Commit: {}</span>", &c[..7.min(c.len())]))
                .unwrap_or_default(),
            self.git_branch
                .as_ref()
                .map(|b| format!("<span>Branch: {}</span>", b))
                .unwrap_or_default(),
            self.build_number
                .as_ref()
                .map(|n| format!("<span>Build: {}</span>", n))
                .unwrap_or_default(),
        )
    }

    fn html_summary(&self, stats: &CategoryStats) -> String {
        format!(
            r#"
        <div class="summary-cards">
            <div class="card">
                <h3>Total Tests</h3>
                <div class="value total">{}</div>
            </div>
            <div class="card">
                <h3>Passed</h3>
                <div class="value passed">{}</div>
            </div>
            <div class="card">
                <h3>Failed</h3>
                <div class="value failed">{}</div>
            </div>
            <div class="card">
                <h3>Skipped</h3>
                <div class="value skipped">{}</div>
            </div>
            <div class="card">
                <h3>Pass Rate</h3>
                <div class="value passed">{:.1}%</div>
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {:.1}%"></div>
                </div>
            </div>
            <div class="card">
                <h3>Duration</h3>
                <div class="value total">{:.2}s</div>
            </div>
        </div>
"#,
            stats.total,
            stats.passed,
            stats.failed,
            stats.skipped,
            stats.pass_rate(),
            stats.pass_rate(),
            stats.total_duration.as_secs_f64(),
        )
    }

    fn html_categories(&self, by_category: &HashMap<String, CategoryStats>) -> String {
        let mut html = String::new();
        html.push_str(r#"<h2 style="margin-bottom: 20px; color: var(--steam-light-blue);">Categories Overview</h2>"#);
        html.push_str(r#"<div class="summary-cards" style="margin-bottom: 30px;">"#);

        let mut categories: Vec<_> = by_category.iter().collect();
        categories.sort_by(|a, b| a.0.cmp(b.0));

        for (name, stats) in categories {
            html.push_str(&format!(
                r#"
            <div class="card">
                <h3>{}</h3>
                <div class="value {}">{}/{}</div>
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {:.1}%"></div>
                </div>
            </div>
"#,
                name,
                if stats.failed > 0 { "failed" } else { "passed" },
                stats.passed,
                stats.total,
                stats.pass_rate(),
            ));
        }

        html.push_str("</div>");
        html
    }

    fn html_results(&self, by_category: &HashMap<String, Vec<&TestResult>>) -> String {
        let mut html = String::new();
        html.push_str(r#"<h2 style="margin-bottom: 20px; color: var(--steam-light-blue);">Detailed Results</h2>"#);

        let mut categories: Vec<_> = by_category.iter().collect();
        categories.sort_by(|a, b| a.0.cmp(b.0));

        for (category, results) in categories {
            let passed = results
                .iter()
                .filter(|r| r.status == TestStatus::Passed)
                .count();
            let failed = results
                .iter()
                .filter(|r| r.status == TestStatus::Failed)
                .count();
            let skipped = results
                .iter()
                .filter(|r| r.status == TestStatus::Skipped)
                .count();

            html.push_str(&format!(
                r#"
        <div class="category-section">
            <div class="category-header">
                <h2>{}</h2>
                <div class="category-stats">
                    <span class="stat passed">{} passed</span>
                    <span class="stat failed">{} failed</span>
                    <span class="stat skipped">{} skipped</span>
                </div>
            </div>
            <div class="test-list">
"#,
                category, passed, failed, skipped
            ));

            for result in results {
                html.push_str(&self.html_test_item(result));
            }

            html.push_str("</div></div>");
        }

        html
    }

    fn html_test_item(&self, result: &TestResult) -> String {
        let mut html = format!(
            r#"
                <div class="test-item">
                    <div class="test-status {}">{}</div>
                    <div class="test-id">{}</div>
                    <div class="test-info">
                        <div class="test-name">{}</div>
                        <div class="test-description">{}</div>
                        {}
                    </div>
                    <div class="test-priority {}">{}</div>
                    <div class="test-duration">{:.3}ms</div>
"#,
            result.status.css_class(),
            result.status.icon(),
            result.id,
            result.name,
            result.description,
            result
                .doc_reference
                .as_ref()
                .map(|url| format!(
                    r#"<a href="{}" class="doc-link" target="_blank">📖 Valve Docs</a>"#,
                    url
                ))
                .unwrap_or_default(),
            result.priority.css_class(),
            result.priority.label(),
            result.duration.as_secs_f64() * 1000.0,
        );

        if let Some(ref error) = result.error_message {
            html.push_str(&format!(r#"<div class="test-error">{}</div>"#, error));
        }

        html.push_str("</div>");
        html
    }

    fn html_footer(&self) -> String {
        r#"
    </div>
    <footer>
        <p>Source Engine Parity Test Suite</p>
        <p>Generated by engine_shared test framework</p>
    </footer>
</body>
</html>
"#
        .to_string()
    }

    /// Save report to file.
    pub fn save_html(&self, path: &Path) -> std::io::Result<()> {
        let html = self.to_html();
        fs::write(path, html)
    }

    /// Save report as JSON.
    pub fn save_json(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json)
    }
}

fn chrono_format(timestamp: u64) -> String {
    // Simple formatting without external chrono crate
    let secs = timestamp;
    let days_since_epoch = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Approximate date (not accurate but works for display)
    let years = 1970 + (days_since_epoch / 365);
    let days = days_since_epoch % 365;
    let month = (days / 30) + 1;
    let day = (days % 30) + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        years, month, day, hours, minutes, seconds
    )
}

/// Test report builder with fluent API.
pub struct ReportBuilder {
    report: TestReport,
}

impl ReportBuilder {
    pub fn new(title: &str) -> Self {
        ReportBuilder {
            report: TestReport::new(title, ""),
        }
    }

    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.report.subtitle = subtitle.to_string();
        self
    }

    pub fn git_info(mut self, commit: Option<&str>, branch: Option<&str>) -> Self {
        self.report.git_commit = commit.map(|s| s.to_string());
        self.report.git_branch = branch.map(|s| s.to_string());
        self
    }

    pub fn build_number(mut self, number: &str) -> Self {
        self.report.build_number = Some(number.to_string());
        self
    }

    pub fn coverage(mut self, percent: f64) -> Self {
        self.report.coverage_percent = Some(percent);
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.report
            .metadata
            .insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_test(mut self, result: TestResult) -> Self {
        self.report.add_result(result);
        self
    }

    pub fn build(self) -> TestReport {
        self.report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let report = ReportBuilder::new("Steam ID Tests")
            .subtitle("Source Engine Parity Suite")
            .add_test(
                TestResult::new("SID-001", "SteamID64 Parsing", "Steam ID")
                    .with_description("Parse 64-bit Steam ID correctly")
                    .with_priority(TestPriority::Critical)
                    .with_doc_reference("https://developer.valvesoftware.com/wiki/SteamID")
                    .pass(Duration::from_millis(5)),
            )
            .add_test(
                TestResult::new("SID-002", "SteamID32 Conversion", "Steam ID")
                    .with_description("Convert between 32-bit and 64-bit formats")
                    .with_priority(TestPriority::Critical)
                    .pass(Duration::from_millis(3)),
            )
            .add_test(
                TestResult::new("AUTH-001", "Valid Steam Login", "Authentication")
                    .with_description("Client authenticates with valid Steam credentials")
                    .with_priority(TestPriority::Critical)
                    .with_doc_reference("https://partner.steamgames.com/doc/features/auth")
                    .pass(Duration::from_millis(10)),
            )
            .build();

        assert_eq!(report.results.len(), 3);
        assert!(report.all_passed());

        let stats = report.overall_stats();
        assert_eq!(stats.passed, 3);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_failed_report() {
        let report = ReportBuilder::new("Test Suite")
            .add_test(
                TestResult::new("TEST-001", "Passing Test", "Tests").pass(Duration::from_millis(1)),
            )
            .add_test(
                TestResult::new("TEST-002", "Failing Test", "Tests")
                    .fail(Duration::from_millis(2), "Expected 42, got 0"),
            )
            .build();

        assert!(!report.all_passed());

        let stats = report.overall_stats();
        assert_eq!(stats.passed, 1);
        assert_eq!(stats.failed, 1);
    }

    #[test]
    fn test_html_generation() {
        let report = ReportBuilder::new("HTML Test")
            .subtitle("Test HTML generation")
            .add_test(
                TestResult::new("HTML-001", "Generate HTML", "HTML").pass(Duration::from_millis(1)),
            )
            .build();

        let html = report.to_html();

        assert!(html.contains("HTML Test"));
        assert!(html.contains("HTML-001"));
        assert!(html.contains("Generate HTML"));
        assert!(html.contains("passed"));
    }

    #[test]
    fn test_category_stats() {
        let report = ReportBuilder::new("Category Test")
            .add_test(TestResult::new("A-001", "Test A1", "Category A").pass(Duration::ZERO))
            .add_test(TestResult::new("A-002", "Test A2", "Category A").pass(Duration::ZERO))
            .add_test(TestResult::new("B-001", "Test B1", "Category B").pass(Duration::ZERO))
            .add_test(TestResult::new("B-002", "Test B2", "Category B").fail(Duration::ZERO, "error"))
            .build();

        let by_cat = report.stats_by_category();

        assert_eq!(by_cat["Category A"].passed, 2);
        assert_eq!(by_cat["Category A"].failed, 0);
        assert_eq!(by_cat["Category B"].passed, 1);
        assert_eq!(by_cat["Category B"].failed, 1);
    }
}
