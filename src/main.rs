use clap::Parser;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "git-stats")]
#[command(about = "Generate git commit statistics per contributor (commits & lines changed)")]
struct Args {
    #[arg(short, long, help = "Branch to analyze")]
    branch: Option<String>,

    #[arg(short, long, help = "Start date (e.g., 2025-01-01)")]
    since: Option<String>,

    #[arg(short, long, help = "End date (e.g., 2025-12-31)")]
    until: Option<String>,

    #[arg(short, long, action = clap::ArgAction::Append, help = "Merge authors (format: Alias1,Alias2 into first name)")]
    merge: Vec<String>,

    #[arg(short, long, help = "Show visual graph of contributions")]
    graph: bool,

    #[arg(long, help = "Generate HTML report with commits/lines toggle, period selector, and charts")]
    html: Option<Option<String>>,

    #[arg(short, long, help = "Open HTML report after generation (optionally specify app, e.g. 'Safari', 'Firefox')")]
    open: Option<Option<String>>,

    #[arg(long, help = "Sort by: commits (default) or lines")]
    sort: Option<String>,
}

struct ContributorStats {
    commits: u64,
    lines_added: u64,
    lines_deleted: u64,
}

fn get_current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_authors(branch: &str, since: &Option<String>, until: &Option<String>) -> Vec<String> {
    let mut args = vec!["log".to_string(), branch.to_string(), "--format=%aN".to_string()];

    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .expect("Failed to execute git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut authors: Vec<String> = stdout
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    authors.sort();
    authors.dedup();
    authors
}

fn get_commit_count(
    branch: &str,
    author: &str,
    since: &Option<String>,
    until: &Option<String>,
) -> u64 {
    let mut args = vec![
        "log".to_string(),
        branch.to_string(),
        format!("--author={}", author),
        "--oneline".to_string(),
    ];

    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .expect("Failed to execute git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().filter(|s| !s.is_empty()).count() as u64
}

fn get_line_stats(
    branch: &str,
    author: &str,
    since: &Option<String>,
    until: &Option<String>,
) -> (u64, u64) {
    let mut args = vec![
        "log".to_string(),
        branch.to_string(),
        format!("--author={}", author),
        "--pretty=tformat:".to_string(),
        "--numstat".to_string(),
    ];

    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .expect("Failed to execute git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut added: u64 = 0;
    let mut deleted: u64 = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(a) = parts[0].parse::<u64>() {
                added += a;
            }
            if let Ok(d) = parts[1].parse::<u64>() {
                deleted += d;
            }
        }
    }

    (added, deleted)
}

fn build_author_mapping(merge_args: &[String]) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    for m in merge_args {
        if let Some((alias, canonical)) = m.split_once('=') {
            mapping.insert(alias.to_string(), canonical.to_string());
        }
    }
    mapping
}

fn get_commits_by_date(
    branch: &str,
    author: Option<&str>,
    since: &Option<String>,
    until: &Option<String>,
) -> BTreeMap<String, u64> {
    let mut args = vec![
        "log".to_string(),
        branch.to_string(),
        "--format=%ad".to_string(),
        "--date=short".to_string(),
    ];

    if let Some(a) = author {
        args.push(format!("--author={}", a));
    }
    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .expect("Failed to execute git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();

    for line in stdout.lines() {
        if !line.is_empty() {
            *counts.entry(line.to_string()).or_insert(0) += 1;
        }
    }

    counts
}

fn get_lines_by_date(
    branch: &str,
    author: Option<&str>,
    since: &Option<String>,
    until: &Option<String>,
) -> BTreeMap<String, u64> {
    let mut args = vec![
        "log".to_string(),
        branch.to_string(),
        "--format=%ad".to_string(),
        "--date=short".to_string(),
        "--numstat".to_string(),
    ];

    if let Some(a) = author {
        args.push(format!("--author={}", a));
    }
    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .expect("Failed to execute git log");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut current_date: Option<String> = None;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Date lines are in format YYYY-MM-DD
        if trimmed.len() == 10 && trimmed.chars().nth(4) == Some('-') && trimmed.chars().nth(7) == Some('-') {
            current_date = Some(trimmed.to_string());
        } else if let Some(ref date) = current_date {
            // numstat lines: added<tab>deleted<tab>filename
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Ok(added), Ok(deleted)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    *counts.entry(date.clone()).or_insert(0) += added + deleted;
                }
            }
        }
    }

    counts
}

fn print_time_graph(title: &str, data: &BTreeMap<String, u64>) {
    if data.is_empty() {
        println!("{}: No data", title);
        println!();
        return;
    }

    let height = 8;
    let width = data.len().min(60);

    println!("{}", title);
    println!("{}", "─".repeat(title.len()));

    let values: Vec<u64> = if data.len() > width {
        let chunk_size = (data.len() + width - 1) / width;
        data.values()
            .collect::<Vec<_>>()
            .chunks(chunk_size)
            .map(|chunk| chunk.iter().copied().sum())
            .collect()
    } else {
        data.values().copied().collect()
    };

    let max_val = *values.iter().max().unwrap_or(&1);

    for row in (0..height).rev() {
        let threshold = (row as f64 / height as f64) * max_val as f64;
        for &val in &values {
            if val as f64 > threshold {
                print!("█");
            } else {
                print!(" ");
            }
        }
        if row == height - 1 {
            println!(" {}", max_val);
        } else if row == 0 {
            println!(" 0");
        } else {
            println!();
        }
    }

    let dates: Vec<&String> = data.keys().collect();
    if let (Some(first), Some(last)) = (dates.first(), dates.last()) {
        println!("{:<width$}{}", first, last, width = values.len().saturating_sub(last.len()));
    }
    println!();
}

fn generate_html_report(
    branch: &str,
    sorted_stats: &[(&String, &ContributorStats)],
    author_mapping: &HashMap<String, String>,
    since: &Option<String>,
    until: &Option<String>,
    output_path: &str,
) -> std::io::Result<()> {
    let mut file = File::create(output_path)?;

    let reverse_mapping: HashMap<&String, Vec<&String>> = {
        let mut map: HashMap<&String, Vec<&String>> = HashMap::new();
        for (alias, canonical) in author_mapping {
            map.entry(canonical).or_default().push(alias);
        }
        map
    };

    let mut weekly_commits: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut weekly_lines: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut total_weekly_commits: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_weekly_lines: BTreeMap<String, u64> = BTreeMap::new();

    for (canonical_name, _) in sorted_stats {
        let mut authors_to_query: Vec<&str> = vec![canonical_name.as_str()];
        if let Some(aliases) = reverse_mapping.get(canonical_name) {
            for alias in aliases {
                authors_to_query.push(alias.as_str());
            }
        }

        let mut combined_commits: BTreeMap<String, u64> = BTreeMap::new();
        let mut combined_lines: BTreeMap<String, u64> = BTreeMap::new();
        for author in authors_to_query {
            let commits_data = get_commits_by_date(branch, Some(author), since, until);
            for (date, count) in commits_data {
                *combined_commits.entry(date.clone()).or_insert(0) += count;
                *total_weekly_commits.entry(date).or_insert(0) += count;
            }
            let lines_data = get_lines_by_date(branch, Some(author), since, until);
            for (date, count) in lines_data {
                *combined_lines.entry(date.clone()).or_insert(0) += count;
                *total_weekly_lines.entry(date).or_insert(0) += count;
            }
        }
        weekly_commits.insert(canonical_name.to_string(), combined_commits);
        weekly_lines.insert(canonical_name.to_string(), combined_lines);
    }

    let colors = ["#58a6ff", "#3fb950", "#f0883e", "#a371f7", "#f85149", "#8b949e"];

    let contributors_json: Vec<String> = sorted_stats
        .iter()
        .enumerate()
        .map(|(i, (name, stats))| {
            let commits = weekly_commits.get(*name).cloned().unwrap_or_default();
            let commits_json: String = commits
                .iter()
                .map(|(date, count)| format!("{{\"date\":\"{}\",\"count\":{}}}", date, count))
                .collect::<Vec<_>>()
                .join(",");

            let lines = weekly_lines.get(*name).cloned().unwrap_or_default();
            let lines_json: String = lines
                .iter()
                .map(|(date, count)| format!("{{\"date\":\"{}\",\"count\":{}}}", date, count))
                .collect::<Vec<_>>()
                .join(",");

            format!(
                r#"{{
                    "name": "{}",
                    "commits": {},
                    "added": {},
                    "deleted": {},
                    "color": "{}",
                    "weeklyCommits": [{}],
                    "weeklyLines": [{}]
                }}"#,
                name,
                stats.commits,
                stats.lines_added,
                stats.lines_deleted,
                colors[i % colors.len()],
                commits_json,
                lines_json
            )
        })
        .collect();

    let total_weekly_commits_json: String = total_weekly_commits
        .iter()
        .map(|(date, count)| format!("{{\"date\":\"{}\",\"count\":{}}}", date, count))
        .collect::<Vec<_>>()
        .join(",");

    let total_weekly_lines_json: String = total_weekly_lines
        .iter()
        .map(|(date, count)| format!("{{\"date\":\"{}\",\"count\":{}}}", date, count))
        .collect::<Vec<_>>()
        .join(",");

    let since_display = since.clone().unwrap_or_else(|| "beginning".to_string());
    let until_display = until.clone().unwrap_or_else(|| "now".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Contributors - {branch}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/chartjs-adapter-date-fns"></script>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 24px;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{ font-size: 24px; font-weight: 600; margin-bottom: 8px; }}
        .subtitle {{ color: #8b949e; font-size: 14px; margin-bottom: 24px; }}
        .card {{
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 16px;
            margin-bottom: 16px;
        }}
        .card-title {{ font-size: 14px; font-weight: 600; margin-bottom: 16px; color: #c9d1d9; }}
        .main-chart {{ height: 200px; }}
        .contributors-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 16px; }}
        .contributor-card {{
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 16px;
        }}
        .contributor-header {{ display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }}
        .avatar {{
            width: 48px; height: 48px;
            border-radius: 50%;
            display: flex; align-items: center; justify-content: center;
            font-weight: 600; font-size: 18px; color: white;
        }}
        .contributor-info h3 {{ font-size: 16px; font-weight: 600; }}
        .contributor-stats {{ font-size: 12px; color: #8b949e; margin-top: 4px; }}
        .contributor-stats .added {{ color: #3fb950; }}
        .contributor-stats .deleted {{ color: #f85149; }}
        .rank {{
            margin-left: auto;
            background: #21262d;
            padding: 4px 8px;
            border-radius: 20px;
            font-size: 12px;
            color: #8b949e;
        }}
        .mini-chart {{ height: 300px; margin-top: 12px; }}
        canvas {{ width: 100% !important; }}
        .period-toggle {{
            display: flex;
            gap: 0;
            margin-bottom: 16px;
        }}
        .period-toggle button {{
            background: #21262d;
            border: 1px solid #30363d;
            color: #c9d1d9;
            padding: 6px 16px;
            font-size: 12px;
            cursor: pointer;
            transition: all 0.2s;
        }}
        .period-toggle button:first-child {{
            border-radius: 6px 0 0 6px;
        }}
        .period-toggle button:last-child {{
            border-radius: 0 6px 6px 0;
        }}
        .period-toggle button:not(:first-child) {{
            border-left: none;
        }}
        .period-toggle button:hover {{
            background: #30363d;
        }}
        .period-toggle button.active {{
            background: #58a6ff;
            border-color: #58a6ff;
            color: #0d1117;
        }}
        .card-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 16px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Contributors</h1>
        <div class="subtitle">Contributions to {branch} · {since_display} to {until_display}</div>

        <div class="card">
            <div class="card-header">
                <div class="card-title" style="margin-bottom: 0;" id="chartTitle">Commits over time</div>
                <div style="display: flex; gap: 16px;">
                    <div class="period-toggle metric-toggle">
                        <button class="active" data-metric="commits">Commits</button>
                        <button data-metric="lines">Lines</button>
                    </div>
                    <div class="period-toggle">
                        <button class="active" data-period="1">1 Day</button>
                        <button data-period="3">3 Days</button>
                        <button data-period="7">1 Week</button>
                        <button data-period="30">1 Month</button>
                        <button data-period="365">1 Year</button>
                    </div>
                </div>
            </div>
            <div class="main-chart">
                <canvas id="mainChart"></canvas>
            </div>
        </div>

        <div class="contributors-grid" id="contributorsGrid"></div>
    </div>

    <script>
    const totalWeeklyCommits = [{total_weekly_commits_json}];
    const totalWeeklyLines = [{total_weekly_lines_json}];
    const contributors = [{contributors_json}];

    // Calculate global bounds from both datasets
    const allCommitDates = totalWeeklyCommits.map(d => d.date);
    const allLinesDates = totalWeeklyLines.map(d => d.date);
    const allDates = [...new Set([...allCommitDates, ...allLinesDates])].sort();
    const globalMinDate = allDates[0];
    const globalMaxDate = allDates[allDates.length - 1];

    // Current metric state
    let currentMetric = 'commits';
    const getTotalWeekly = () => currentMetric === 'commits' ? totalWeeklyCommits : totalWeeklyLines;
    const getContribWeekly = (contrib) => currentMetric === 'commits' ? contrib.weeklyCommits : contrib.weeklyLines;

    // Calculate totals for percentages
    const totalCommits = contributors.reduce((sum, c) => sum + c.commits, 0);
    const totalLines = contributors.reduce((sum, c) => sum + c.added + c.deleted, 0);

    // Aggregation function
    function aggregateByPeriod(data, days) {{
        if (days === 1) return data.map(d => ({{ x: d.date, y: d.count }}));

        const sorted = [...data].sort((a, b) => a.date.localeCompare(b.date));
        const buckets = {{}};

        sorted.forEach(d => {{
            const date = new Date(d.date);
            const bucketStart = new Date(date);
            bucketStart.setDate(bucketStart.getDate() - (bucketStart.getDate() % days));
            const key = bucketStart.toISOString().split('T')[0];
            buckets[key] = (buckets[key] || 0) + d.count;
        }});

        return Object.entries(buckets)
            .map(([date, count]) => ({{ x: date, y: count }}))
            .sort((a, b) => a.x.localeCompare(b.x));
    }}

    // Pad data with boundary points
    function padData(data, minDate, maxDate) {{
        const result = [...data];
        if (result.length === 0 || result[0].x > minDate) {{
            result.unshift({{ x: minDate, y: 0 }});
        }}
        if (result.length === 0 || result[result.length - 1].x < maxDate) {{
            result.push({{ x: maxDate, y: 0 }});
        }}
        return result;
    }}

    // Fill data to match all dates from reference (for proper stacking)
    function fillToAllDates(data, referenceDates) {{
        const dataMap = new Map(data.map(d => [d.x, d.y]));
        return referenceDates.map(date => ({{
            x: date,
            y: dataMap.get(date) || 0
        }}));
    }}

    // Calculate max for current period
    function getGlobalMax(period) {{
        const totalAgg = aggregateByPeriod(getTotalWeekly(), period);
        return Math.max(...totalAgg.map(d => d.y), 1);
    }}

    let currentPeriod = 1;
    let mainChart, contribCharts = [];

    // Main chart - stacked area with all contributors (reversed: smallest at bottom, largest at top)
    const mainCtx = document.getElementById('mainChart').getContext('2d');
    const allDatesForPeriod = (period) => aggregateByPeriod(getTotalWeekly(), period).map(d => d.x);
    const contributorsReversed = [...contributors].reverse();
    const mainDatasets = contributorsReversed.map((contrib, i) => ({{
        label: contrib.name,
        data: fillToAllDates(aggregateByPeriod(getContribWeekly(contrib), 1), allDatesForPeriod(1)),
        borderColor: contrib.color,
        backgroundColor: contrib.color + '80',
        fill: 'origin',
        tension: 0.03,
        pointRadius: 0,
        pointHoverRadius: 4,
        pointHoverBackgroundColor: contrib.color,
        borderWidth: 1
    }}));

    mainChart = new Chart(mainCtx, {{
        type: 'line',
        data: {{ datasets: mainDatasets }},
        options: {{
            responsive: true,
            maintainAspectRatio: false,
            interaction: {{
                intersect: false,
                mode: 'index'
            }},
            plugins: {{
                legend: {{ display: false }},
                tooltip: {{
                    backgroundColor: '#161b22',
                    borderColor: '#30363d',
                    borderWidth: 1,
                    titleColor: '#c9d1d9',
                    bodyColor: '#c9d1d9',
                    padding: 12,
                    displayColors: true,
                    callbacks: {{
                        title: (items) => items[0]?.label || '',
                        label: (item) => `${{item.dataset.label}}: ${{item.parsed.y}} ${{currentMetric === 'commits' ? 'commits' : 'lines'}}`
                    }}
                }}
            }},
            scales: {{
                x: {{
                    type: 'time',
                    time: {{ unit: 'month' }},
                    min: globalMinDate,
                    max: globalMaxDate,
                    grid: {{ color: '#21262d' }},
                    ticks: {{ color: '#8b949e' }}
                }},
                y: {{
                    stacked: true,
                    beginAtZero: true,
                    grid: {{ color: '#21262d' }},
                    ticks: {{ color: '#8b949e', precision: 0 }}
                }}
            }}
        }}
    }});

    // Contributor cards
    const grid = document.getElementById('contributorsGrid');

    contributors.forEach((contrib, index) => {{
        const initials = contrib.name.split(' ').map(n => n[0]).join('').toUpperCase();
        const card = document.createElement('div');
        card.className = 'contributor-card';
        card.innerHTML = `
            <div class="contributor-header">
                <div class="avatar" style="background: ${{contrib.color}};">${{initials}}</div>
                <div class="contributor-info">
                    <h3>${{contrib.name}}</h3>
                    <div class="contributor-stats">
                        <span id="metric-${{index}}">${{contrib.commits.toLocaleString()}} commits</span>
                        <span id="percent-${{index}}" style="color: #8b949e; margin-left: 8px;">${{(contrib.commits / totalCommits * 100).toFixed(1)}}%</span> &nbsp;
                        <span class="added">${{contrib.added.toLocaleString()}} ++</span> &nbsp;
                        <span class="deleted">${{contrib.deleted.toLocaleString()}} --</span>
                    </div>
                </div>
                <span class="rank">#${{index + 1}}</span>
            </div>
            <div class="mini-chart">
                <canvas id="chart-${{index}}"></canvas>
            </div>
        `;
        grid.appendChild(card);

        const contribData = fillToAllDates(aggregateByPeriod(getContribWeekly(contrib), 1), allDatesForPeriod(1));

        // Mini chart
        const chart = new Chart(document.getElementById(`chart-${{index}}`).getContext('2d'), {{
            type: 'line',
            data: {{
                datasets: [{{
                    data: contribData,
                    borderColor: contrib.color,
                    backgroundColor: contrib.color + '20',
                    fill: true,
                    tension: 0.03,
                    pointRadius: 0,
                    pointHoverRadius: 5,
                    pointHoverBackgroundColor: contrib.color,
                    borderWidth: 2
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                interaction: {{
                    intersect: false,
                    mode: 'index'
                }},
                plugins: {{
                    legend: {{ display: false }},
                    tooltip: {{
                        backgroundColor: '#161b22',
                        borderColor: '#30363d',
                        borderWidth: 1,
                        titleColor: '#c9d1d9',
                        bodyColor: '#c9d1d9',
                        padding: 10,
                        displayColors: false,
                        callbacks: {{
                            title: (items) => items[0]?.label || '',
                            label: (item) => `${{item.parsed.y}} ${{currentMetric === 'commits' ? 'commits' : 'lines'}}`
                        }}
                    }}
                }},
                scales: {{
                    x: {{
                        type: 'time',
                        time: {{ unit: 'month' }},
                        min: globalMinDate,
                        max: globalMaxDate,
                        grid: {{ display: false }},
                        ticks: {{ color: '#8b949e', maxTicksLimit: 4 }}
                    }},
                    y: {{
                        beginAtZero: true,
                        min: -getGlobalMax(1) * 0.05,
                        max: getGlobalMax(1),
                        grid: {{ display: false }},
                        ticks: {{ display: false }}
                    }}
                }}
            }}
        }});
        contribCharts.push({{ chart, contrib }});
    }});

    // Period toggle handler
    function updateCharts(period) {{
        currentPeriod = period;
        const globalMax = getGlobalMax(period);
        const dates = allDatesForPeriod(period);

        // Update main chart (all contributor datasets)
        contributorsReversed.forEach((contrib, i) => {{
            mainChart.data.datasets[i].data = fillToAllDates(aggregateByPeriod(getContribWeekly(contrib), period), dates);
        }});
        mainChart.update();

        // Update contributor charts
        contribCharts.forEach(({{ chart, contrib }}) => {{
            const newData = fillToAllDates(aggregateByPeriod(getContribWeekly(contrib), period), dates);
            chart.data.datasets[0].data = newData;
            chart.options.scales.y.max = globalMax;
            chart.options.scales.y.min = -globalMax * 0.05;
            chart.update();
        }});
    }}

    // Period toggle button click handlers
    document.querySelectorAll('.period-toggle:not(.metric-toggle) button').forEach(btn => {{
        btn.addEventListener('click', () => {{
            document.querySelectorAll('.period-toggle:not(.metric-toggle) button').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            updateCharts(parseInt(btn.dataset.period));
        }});
    }});

    // Metric toggle button click handlers
    document.querySelectorAll('.metric-toggle button').forEach(btn => {{
        btn.addEventListener('click', () => {{
            document.querySelectorAll('.metric-toggle button').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            currentMetric = btn.dataset.metric;
            document.getElementById('chartTitle').textContent = currentMetric === 'commits' ? 'Commits over time' : 'Lines changed over time';
            // Update contributor card stats and percentages
            contributors.forEach((contrib, index) => {{
                const value = currentMetric === 'commits' ? contrib.commits : (contrib.added + contrib.deleted);
                const total = currentMetric === 'commits' ? totalCommits : totalLines;
                const percent = (value / total * 100).toFixed(1);
                const label = currentMetric === 'commits' ? 'commits' : 'lines';
                document.getElementById(`metric-${{index}}`).textContent = `${{value.toLocaleString()}} ${{label}}`;
                document.getElementById(`percent-${{index}}`).textContent = `${{percent}}%`;
            }});
            updateCharts(currentPeriod);
        }});
    }});

    // Set default: Lines metric and 1 Week period
    document.querySelector('.metric-toggle button[data-metric="lines"]').click();
    document.querySelector('.period-toggle:not(.metric-toggle) button[data-period="7"]').click();
    </script>
</body>
</html>"#,
        branch = branch,
        since_display = since_display,
        until_display = until_display,
        total_weekly_commits_json = total_weekly_commits_json,
        total_weekly_lines_json = total_weekly_lines_json,
        contributors_json = contributors_json.join(","),
    );

    file.write_all(html.as_bytes())?;
    Ok(())
}

fn main() {
    let args = Args::parse();

    let branch = args.branch.unwrap_or_else(|| {
        get_current_branch().unwrap_or_else(|| "main".to_string())
    });

    let author_mapping = build_author_mapping(&args.merge);

    let raw_authors = get_authors(&branch, &args.since, &args.until);

    let mut stats_by_canonical: HashMap<String, ContributorStats> = HashMap::new();

    for author in &raw_authors {
        let canonical = author_mapping.get(author).unwrap_or(author);

        let commits = get_commit_count(&branch, author, &args.since, &args.until);
        let (added, deleted) = get_line_stats(&branch, author, &args.since, &args.until);

        let entry = stats_by_canonical
            .entry(canonical.clone())
            .or_insert(ContributorStats {
                commits: 0,
                lines_added: 0,
                lines_deleted: 0,
            });

        entry.commits += commits;
        entry.lines_added += added;
        entry.lines_deleted += deleted;
    }

    let sort_by_lines = args.sort.as_deref() == Some("lines");
    let mut sorted_stats: Vec<(&String, &ContributorStats)> = stats_by_canonical.iter().collect();
    if sort_by_lines {
        sorted_stats.sort_by(|a, b| {
            let a_lines = a.1.lines_added + a.1.lines_deleted;
            let b_lines = b.1.lines_added + b.1.lines_deleted;
            b_lines.cmp(&a_lines)
        });
    } else {
        sorted_stats.sort_by(|a, b| b.1.commits.cmp(&a.1.commits));
    }

    println!("Branch: {}", branch);
    println!();

    let name_width = sorted_stats
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(12)
        .max(12);

    let total_commits: u64 = sorted_stats.iter().map(|(_, s)| s.commits).sum();
    let total_added: u64 = sorted_stats.iter().map(|(_, s)| s.lines_added).sum();
    let total_deleted: u64 = sorted_stats.iter().map(|(_, s)| s.lines_deleted).sum();
    let total_lines: u64 = total_added + total_deleted;

    if args.html.is_some() {
        let output_path = args
            .html
            .as_ref()
            .and_then(|o| o.clone())
            .unwrap_or_else(|| "contrib-report.html".to_string());

        match generate_html_report(
            &branch,
            &sorted_stats,
            &author_mapping,
            &args.since,
            &args.until,
            &output_path,
        ) {
            Ok(_) => {
                println!("HTML report generated: {}", output_path);
                if args.open.is_some() {
                    let app = args.open.as_ref().and_then(|o| o.clone());
                    #[cfg(target_os = "macos")]
                    {
                        if let Some(app_name) = app {
                            let _ = Command::new("open")
                                .args(["-a", &app_name, &output_path])
                                .spawn();
                        } else {
                            let _ = Command::new("open")
                                .args(["-a", "Safari", &output_path])
                                .spawn();
                        }
                    }
                    #[cfg(target_os = "linux")]
                    {
                        if let Some(app_name) = app {
                            let _ = Command::new(&app_name).arg(&output_path).spawn();
                        } else {
                            let _ = Command::new("xdg-open").arg(&output_path).spawn();
                        }
                    }
                    #[cfg(target_os = "windows")]
                    {
                        if let Some(app_name) = app {
                            let _ = Command::new("cmd")
                                .args(["/C", "start", &app_name, &output_path])
                                .spawn();
                        } else {
                            let _ = Command::new("cmd")
                                .args(["/C", "start", &output_path])
                                .spawn();
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error generating HTML report: {}", e),
        }
    } else if args.graph {
        let team_data = get_commits_by_date(&branch, None, &args.since, &args.until);
        print_time_graph("Team (all contributors)", &team_data);

        for (name, _) in &sorted_stats {
            let author_data = get_commits_by_date(&branch, Some(name), &args.since, &args.until);
            print_time_graph(name, &author_data);
        }
    } else {
        println!(
            "| {:<name_width$} | {:>8} | {:>15} | {:>17} | {:>6} |",
            "Contributeur", "Commits", "Lignes ajoutees", "Lignes supprimees", "%",
            name_width = name_width
        );
        println!(
            "|{:-<width$}|{:-<10}|{:-<17}|{:-<19}|{:-<8}|",
            "", "", "", "", "",
            width = name_width + 2
        );

        for (name, stat) in &sorted_stats {
            let pct = if sort_by_lines {
                let lines = stat.lines_added + stat.lines_deleted;
                if total_lines > 0 { (lines as f64 / total_lines as f64) * 100.0 } else { 0.0 }
            } else {
                if total_commits > 0 { (stat.commits as f64 / total_commits as f64) * 100.0 } else { 0.0 }
            };
            println!(
                "| {:<name_width$} | {:>8} | {:>15} | {:>17} | {:>5.1}% |",
                name,
                stat.commits,
                stat.lines_added,
                stat.lines_deleted,
                pct,
                name_width = name_width
            );
        }

        println!(
            "|{:-<width$}|{:-<10}|{:-<17}|{:-<19}|{:-<8}|",
            "", "", "", "", "",
            width = name_width + 2
        );
        println!(
            "| {:<name_width$} | {:>8} | {:>15} | {:>17} | {:>6} |",
            "TOTAL",
            total_commits,
            total_added,
            total_deleted,
            "100%",
            name_width = name_width
        );
    }
}
