# git-contrib-stats

CLI tool to generate git commit statistics per contributor.

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
| `-m, --merge <MAPPING>` | Merge authors (format: Alias=CanonicalName) |

### Example

```bash
git-contrib-stats -b main \
  -m "john.doe=John" \
  -m "JohnD=John" \
  -m "jane_smith=Jane"
```

### Output

```
Branch: main

| Contributeur |  Commits | Lignes ajoutees | Lignes supprimees |
|--------------|----------|-----------------|-------------------|
| John         |      142 |           15234 |              8721 |
| Jane         |       87 |            9876 |              4532 |
| Bob          |       45 |            3210 |              1234 |
|--------------|----------|-----------------|-------------------|
| TOTAL        |      274 |           28320 |             14487 |
```

## License

MIT
