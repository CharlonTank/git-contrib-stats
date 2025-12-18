# git-contrib-stats

CLI tool to generate git commit statistics per contributor with optional GitHub-style HTML reports.

## Installation

```bash
cargo install git-contrib-stats
```

## Usage

```bash
git-contrib-stats [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-b, --branch <BRANCH>` | Branch to analyze (default: current branch) |
| `-s, --since <DATE>` | Start date (e.g., 2025-01-01) |
| `-u, --until <DATE>` | End date (e.g., 2025-12-31) |
| `-m, --merge <MAPPING>` | Merge authors (format: Alias1,Alias2,... into first name) |
| `--html` | Generate an HTML report with interactive charts |
| `--open [APP]` | Open the HTML report in browser (optionally specify app: Safari, Chrome, Firefox) |

### Examples

Basic usage:
```bash
git-contrib-stats -b main
```

Merge multiple author aliases:
```bash
git-contrib-stats \
  -m "John,john.doe,JohnD" \
  -m "Jane,jane_smith"
```

Generate HTML report and open in Safari:
```bash
git-contrib-stats --html --open Safari
```

### Terminal Output

```
Branch: main

| Contributor  |  Commits | Lines added | Lines deleted |
|--------------|----------|-------------|---------------|
| John         |      142 |       15234 |          8721 |
| Jane         |       87 |        9876 |          4532 |
| Bob          |       45 |        3210 |          1234 |
|--------------|----------|-------------|---------------|
| TOTAL        |      274 |       28320 |         14487 |
```

### HTML Report

When using `--html`, generates a `contrib-report.html` file featuring:

- **Stacked area chart** showing all contributors over time
- **Individual contributor charts** with commit history
- **Period toggles**: 1 Day, 3 Days, 1 Week, 1 Month, 1 Year
- **Interactive tooltips** with commit counts
- **GitHub dark theme** styling

## License

MIT
