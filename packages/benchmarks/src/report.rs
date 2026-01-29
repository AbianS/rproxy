//! Benchmark results reporting and comparison

use crate::runner::BenchmarkResult;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tabled::{settings::Style, Table, Tabled};

/// Compare two benchmark results
pub fn compare_results(baseline_path: &Path, comparison_path: &Path) -> Result<()> {
    let baseline = BenchmarkResult::load(baseline_path)?;
    let comparison = BenchmarkResult::load(comparison_path)?;

    println!("\n{}\n", "Benchmark Comparison".yellow().bold());
    println!(
        "  {} {} vs {}",
        "Comparing:".dimmed(),
        baseline.proxy.cyan(),
        comparison.proxy.cyan()
    );
    println!("  {} {}", "Scenario:".dimmed(), baseline.scenario);
    println!();

    // Throughput comparison
    let throughput_change = percentage_change(
        baseline.results.throughput.requests_per_second,
        comparison.results.throughput.requests_per_second,
    );
    print_metric_comparison(
        "Throughput (req/s)",
        baseline.results.throughput.requests_per_second,
        comparison.results.throughput.requests_per_second,
        throughput_change,
        true, // Higher is better
    );

    // Latency comparison (p99)
    let latency_change = percentage_change(
        baseline.results.latency_ms.p99,
        comparison.results.latency_ms.p99,
    );
    print_metric_comparison(
        "Latency P99 (ms)",
        baseline.results.latency_ms.p99,
        comparison.results.latency_ms.p99,
        latency_change,
        false, // Lower is better
    );

    // Memory comparison
    let memory_change = percentage_change(
        baseline.results.memory_mb.peak,
        comparison.results.memory_mb.peak,
    );
    print_metric_comparison(
        "Memory Peak (MB)",
        baseline.results.memory_mb.peak,
        comparison.results.memory_mb.peak,
        memory_change,
        false, // Lower is better
    );

    // CPU comparison
    let cpu_change = percentage_change(
        baseline.results.cpu_percent.average,
        comparison.results.cpu_percent.average,
    );
    print_metric_comparison(
        "CPU Average (%)",
        baseline.results.cpu_percent.average,
        comparison.results.cpu_percent.average,
        cpu_change,
        false, // Lower is better
    );

    // Error rate comparison
    let baseline_error_rate = calculate_error_rate(&baseline);
    let comparison_error_rate = calculate_error_rate(&comparison);
    let error_change = percentage_change(baseline_error_rate, comparison_error_rate);
    print_metric_comparison(
        "Error Rate (%)",
        baseline_error_rate,
        comparison_error_rate,
        error_change,
        false, // Lower is better
    );

    Ok(())
}

/// Show a single benchmark result
pub fn show_result(path: &Path) -> Result<()> {
    let result = BenchmarkResult::load(path)?;

    println!("\n{}\n", "Benchmark Result".yellow().bold());
    println!("  {} {}", "Run ID:".dimmed(), result.run_id);
    println!("  {} {}", "Proxy:".dimmed(), result.proxy.cyan());
    println!("  {} {}", "Scenario:".dimmed(), result.scenario);
    println!("  {} {}", "Timestamp:".dimmed(), result.timestamp);
    println!();

    println!("  {}", "Configuration".green());
    println!(
        "    Duration: {}s | Target RPS: {} | Concurrency: {}",
        result.config.duration_secs, result.config.target_rps, result.config.concurrency
    );
    println!();

    println!("  {}", "Throughput".green());
    println!(
        "    Total: {} | Successful: {} | Failed: {} | RPS: {:.2}",
        result.results.throughput.total_requests,
        result.results.throughput.successful,
        result.results.throughput.failed,
        result.results.throughput.requests_per_second
    );
    println!();

    println!("  {}", "Latency (ms)".green());
    println!(
        "    Min: {:.2} | Max: {:.2} | Mean: {:.2}",
        result.results.latency_ms.min,
        result.results.latency_ms.max,
        result.results.latency_ms.mean
    );
    println!(
        "    P50: {:.2} | P90: {:.2} | P95: {:.2} | P99: {:.2} | P99.9: {:.2}",
        result.results.latency_ms.p50,
        result.results.latency_ms.p90,
        result.results.latency_ms.p95,
        result.results.latency_ms.p99,
        result.results.latency_ms.p999
    );
    println!();

    println!("  {}", "Resources".green());
    println!(
        "    Memory: idle={:.1}MB | peak={:.1}MB | avg={:.1}MB",
        result.results.memory_mb.idle,
        result.results.memory_mb.peak,
        result.results.memory_mb.average
    );
    println!(
        "    CPU: peak={:.1}% | avg={:.1}%",
        result.results.cpu_percent.peak, result.results.cpu_percent.average
    );
    println!(
        "    Network: RX={:.2}MB | TX={:.2}MB",
        result.results.network.rx_mb, result.results.network.tx_mb
    );
    println!();

    if result.results.throughput.failed > 0 {
        println!("  {}", "Errors".red());
        println!(
            "    Timeout: {} | Refused: {} | 4xx: {} | 5xx: {} | Other: {}",
            result.results.errors.timeout,
            result.results.errors.connection_refused,
            result.results.errors.status_4xx,
            result.results.errors.status_5xx,
            result.results.errors.other
        );
    }

    Ok(())
}

/// Print comparison table for multiple proxies
pub fn print_comparison_table(results: &[(String, BenchmarkResult)]) {
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Proxy")]
        proxy: String,
        #[tabled(rename = "RPS")]
        rps: String,
        #[tabled(rename = "P50 (ms)")]
        p50: String,
        #[tabled(rename = "P99 (ms)")]
        p99: String,
        #[tabled(rename = "Memory (MB)")]
        memory: String,
        #[tabled(rename = "CPU (%)")]
        cpu: String,
        #[tabled(rename = "Errors")]
        errors: String,
    }

    let rows: Vec<Row> = results
        .iter()
        .map(|(name, r)| {
            let error_rate = calculate_error_rate(r);
            Row {
                proxy: name.clone(),
                rps: format!("{:.0}", r.results.throughput.requests_per_second),
                p50: format!("{:.2}", r.results.latency_ms.p50),
                p99: format!("{:.2}", r.results.latency_ms.p99),
                memory: format!("{:.1}", r.results.memory_mb.peak),
                cpu: format!("{:.1}", r.results.cpu_percent.average),
                errors: format!("{:.2}%", error_rate),
            }
        })
        .collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{}", table);

    // Find best in each category
    if results.len() > 1 {
        println!();
        print_winner(
            "Highest Throughput",
            results,
            |r| r.results.throughput.requests_per_second,
            true,
        );
        print_winner(
            "Lowest Latency (P99)",
            results,
            |r| r.results.latency_ms.p99,
            false,
        );
        print_winner(
            "Lowest Memory",
            results,
            |r| r.results.memory_mb.peak,
            false,
        );
        print_winner(
            "Lowest CPU",
            results,
            |r| r.results.cpu_percent.average,
            false,
        );
    }
}

/// Generate a full comparison report
pub fn generate_report(input_dir: &Path, output_path: &Path, format: &str) -> Result<()> {
    // Find all result files
    let mut results: Vec<BenchmarkResult> = Vec::new();

    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(result) = BenchmarkResult::load(&path) {
                results.push(result);
            }
        }
    }

    if results.is_empty() {
        anyhow::bail!("No benchmark results found in {}", input_dir.display());
    }

    // Group by scenario
    let mut by_scenario: std::collections::HashMap<String, Vec<&BenchmarkResult>> =
        std::collections::HashMap::new();

    for result in &results {
        by_scenario
            .entry(result.scenario.clone())
            .or_default()
            .push(result);
    }

    let report = match format {
        "markdown" | "md" => generate_markdown_report(&by_scenario),
        "html" => generate_html_report(&by_scenario),
        "json" => generate_json_report(&results)?,
        _ => anyhow::bail!("Unknown format: {}. Use markdown, html, or json.", format),
    };

    std::fs::write(output_path, report)?;
    println!("{} {}", "Report saved to:".green(), output_path.display());

    Ok(())
}

fn generate_markdown_report(
    by_scenario: &std::collections::HashMap<String, Vec<&BenchmarkResult>>,
) -> String {
    let mut md = String::new();

    md.push_str("# Reverse Proxy Benchmark Report\n\n");
    md.push_str(&format!(
        "Generated: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    for (scenario, results) in by_scenario {
        md.push_str(&format!("## Scenario: {}\n\n", scenario));

        // Throughput table
        md.push_str("### Throughput\n\n");
        md.push_str("| Proxy | Requests/sec | Total | Failed | Error Rate |\n");
        md.push_str("|-------|-------------|-------|--------|------------|\n");

        for r in results {
            let error_rate = calculate_error_rate(r);
            md.push_str(&format!(
                "| {} | {:.0} | {} | {} | {:.2}% |\n",
                r.proxy,
                r.results.throughput.requests_per_second,
                r.results.throughput.total_requests,
                r.results.throughput.failed,
                error_rate
            ));
        }

        // Latency table
        md.push_str("\n### Latency (ms)\n\n");
        md.push_str("| Proxy | P50 | P90 | P95 | P99 | P99.9 | Max |\n");
        md.push_str("|-------|-----|-----|-----|-----|-------|-----|\n");

        for r in results {
            md.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                r.proxy,
                r.results.latency_ms.p50,
                r.results.latency_ms.p90,
                r.results.latency_ms.p95,
                r.results.latency_ms.p99,
                r.results.latency_ms.p999,
                r.results.latency_ms.max
            ));
        }

        // Resources table
        md.push_str("\n### Resource Usage\n\n");
        md.push_str("| Proxy | Memory Idle | Memory Peak | Memory Avg | CPU Peak | CPU Avg |\n");
        md.push_str("|-------|-------------|-------------|------------|----------|--------|\n");

        for r in results {
            md.push_str(&format!(
                "| {} | {:.1} MB | {:.1} MB | {:.1} MB | {:.1}% | {:.1}% |\n",
                r.proxy,
                r.results.memory_mb.idle,
                r.results.memory_mb.peak,
                r.results.memory_mb.average,
                r.results.cpu_percent.peak,
                r.results.cpu_percent.average
            ));
        }

        md.push_str("\n---\n\n");
    }

    md
}

fn generate_html_report(
    by_scenario: &std::collections::HashMap<String, Vec<&BenchmarkResult>>,
) -> String {
    let mut html = String::new();

    html.push_str(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Reverse Proxy Benchmark Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }
        h1 { color: #333; }
        h2 { color: #666; border-bottom: 2px solid #ddd; padding-bottom: 10px; }
        table { border-collapse: collapse; width: 100%; margin: 20px 0; background: white; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background: #f8f9fa; font-weight: 600; }
        tr:hover { background: #f8f9fa; }
        .winner { background: #d4edda !important; }
        .metric-good { color: #28a745; }
        .metric-bad { color: #dc3545; }
    </style>
</head>
<body>
    <h1>Reverse Proxy Benchmark Report</h1>
"#,
    );

    html.push_str(&format!(
        "<p>Generated: {}</p>",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    for (scenario, results) in by_scenario {
        html.push_str(&format!("<h2>Scenario: {}</h2>", scenario));

        // Throughput
        html.push_str("<h3>Throughput</h3><table><tr><th>Proxy</th><th>Requests/sec</th><th>Total</th><th>Failed</th><th>Error Rate</th></tr>");
        for r in results {
            let error_rate = calculate_error_rate(r);
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.0}</td><td>{}</td><td>{}</td><td>{:.2}%</td></tr>",
                r.proxy,
                r.results.throughput.requests_per_second,
                r.results.throughput.total_requests,
                r.results.throughput.failed,
                error_rate
            ));
        }
        html.push_str("</table>");

        // Latency
        html.push_str("<h3>Latency (ms)</h3><table><tr><th>Proxy</th><th>P50</th><th>P90</th><th>P95</th><th>P99</th><th>Max</th></tr>");
        for r in results {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td></tr>",
                r.proxy,
                r.results.latency_ms.p50,
                r.results.latency_ms.p90,
                r.results.latency_ms.p95,
                r.results.latency_ms.p99,
                r.results.latency_ms.max
            ));
        }
        html.push_str("</table>");

        // Resources
        html.push_str("<h3>Resource Usage</h3><table><tr><th>Proxy</th><th>Memory Idle</th><th>Memory Peak</th><th>CPU Peak</th><th>CPU Avg</th></tr>");
        for r in results {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.1} MB</td><td>{:.1} MB</td><td>{:.1}%</td><td>{:.1}%</td></tr>",
                r.proxy,
                r.results.memory_mb.idle,
                r.results.memory_mb.peak,
                r.results.cpu_percent.peak,
                r.results.cpu_percent.average
            ));
        }
        html.push_str("</table>");
    }

    html.push_str("</body></html>");
    html
}

fn generate_json_report(results: &[BenchmarkResult]) -> Result<String> {
    Ok(serde_json::to_string_pretty(results)?)
}

// Helper functions

fn percentage_change(baseline: f64, comparison: f64) -> f64 {
    if baseline == 0.0 {
        return 0.0;
    }
    ((comparison - baseline) / baseline) * 100.0
}

fn calculate_error_rate(result: &BenchmarkResult) -> f64 {
    if result.results.throughput.total_requests == 0 {
        return 0.0;
    }
    (result.results.throughput.failed as f64 / result.results.throughput.total_requests as f64)
        * 100.0
}

fn print_metric_comparison(
    name: &str,
    baseline: f64,
    comparison: f64,
    change: f64,
    higher_is_better: bool,
) {
    let is_improvement = if higher_is_better {
        change > 0.0
    } else {
        change < 0.0
    };

    let change_str = if change >= 0.0 {
        format!("+{:.1}%", change)
    } else {
        format!("{:.1}%", change)
    };

    let colored_change = if is_improvement {
        change_str.green()
    } else if change.abs() < 1.0 {
        change_str.dimmed()
    } else {
        change_str.red()
    };

    println!(
        "  {:<20} {:>10.2} -> {:>10.2}  {}",
        name, baseline, comparison, colored_change
    );
}

fn print_winner<F>(
    label: &str,
    results: &[(String, BenchmarkResult)],
    extractor: F,
    higher_is_better: bool,
) where
    F: Fn(&BenchmarkResult) -> f64,
{
    let winner = if higher_is_better {
        results
            .iter()
            .max_by(|a, b| extractor(&a.1).partial_cmp(&extractor(&b.1)).unwrap())
    } else {
        results
            .iter()
            .min_by(|a, b| extractor(&a.1).partial_cmp(&extractor(&b.1)).unwrap())
    };

    if let Some((name, result)) = winner {
        let value = extractor(result);
        println!(
            "  {} {} ({:.2})",
            label.dimmed(),
            name.green().bold(),
            value
        );
    }
}
