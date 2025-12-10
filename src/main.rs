use clap::Parser;
use std::collections::HashMap;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "git-stats")]
#[command(about = "Generate git commit statistics per contributor")]
struct Args {
    #[arg(short, long, help = "Branch to analyze")]
    branch: Option<String>,

    #[arg(short, long, help = "Start date (e.g., 2025-01-01)")]
    since: Option<String>,

    #[arg(short, long, help = "End date (e.g., 2025-12-31)")]
    until: Option<String>,

    #[arg(short, long, action = clap::ArgAction::Append, help = "Merge authors (format: Alias=CanonicalName)")]
    merge: Vec<String>,
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

    let mut sorted_stats: Vec<(&String, &ContributorStats)> = stats_by_canonical.iter().collect();
    sorted_stats.sort_by(|a, b| b.1.commits.cmp(&a.1.commits));

    println!("Branch: {}", branch);
    println!();

    let name_width = sorted_stats
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(12)
        .max(12);

    println!(
        "| {:<name_width$} | {:>8} | {:>15} | {:>17} |",
        "Contributeur", "Commits", "Lignes ajoutees", "Lignes supprimees",
        name_width = name_width
    );
    println!(
        "|{:-<width$}|{:-<10}|{:-<17}|{:-<19}|",
        "", "", "", "",
        width = name_width + 2
    );

    let mut total_commits: u64 = 0;
    let mut total_added: u64 = 0;
    let mut total_deleted: u64 = 0;

    for (name, stat) in &sorted_stats {
        println!(
            "| {:<name_width$} | {:>8} | {:>15} | {:>17} |",
            name,
            stat.commits,
            stat.lines_added,
            stat.lines_deleted,
            name_width = name_width
        );
        total_commits += stat.commits;
        total_added += stat.lines_added;
        total_deleted += stat.lines_deleted;
    }

    println!(
        "|{:-<width$}|{:-<10}|{:-<17}|{:-<19}|",
        "", "", "", "",
        width = name_width + 2
    );
    println!(
        "| {:<name_width$} | {:>8} | {:>15} | {:>17} |",
        "TOTAL",
        total_commits,
        total_added,
        total_deleted,
        name_width = name_width
    );
}
