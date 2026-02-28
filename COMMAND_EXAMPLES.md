# COMMAND_EXAMPLES.md

## Purpose

Golden command examples with expected behavior and exit-code intent.

## Analyze

```bash
harness analyze /path/to/repo --format md
```

Expected:
- read-only behavior
- score/findings output
- exit `0` or `1` (warnings)

```bash
harness analyze /path/to/non-git-repo
```

Expected:
- error: not a git repository
- exit `3`

## Suggest

```bash
harness suggest /path/to/repo
```

Expected:
- ranked recommendations
- exit `0`

```bash
harness suggest /path/to/repo --export-diff
```

Expected:
- plan artifact under `.harness/plans/`
- exit `0`

## Apply

```bash
harness apply /path/to/repo --plan-all --apply-mode preview
```

Expected:
- precondition checks
- scope preview output
- no write in preview mode
- exit `0`

Expected with lifecycle policy:
- if config marks tool only under `tools.deprecated.deprecated`, apply remains allowed
- if config marks tool under `tools.deprecated.disabled`, apply is rejected (exit `3`)

```bash
harness apply /path/to/repo --apply-mode preview
```

Expected:
- CLI selector validation failure (missing plan selector)
- exit `3`

```bash
harness apply /path/to/repo --plan-file ../plan.json --apply-mode preview
```

Expected:
- path traversal rejection
- exit `3`

## Lint

```bash
harness lint /path/to/repo
```

Expected:
- policy/conformance findings
- exit `0`, `1`, or `2` depending severity

Lifecycle-specific expectations:
- `tools.deprecated.observe` only -> warning output including `tools.observe`, exit `1`
- `tools.deprecated.deprecated` present -> blocking output including `tools.deprecated`, exit `2`

## Optimize

```bash
harness optimize /path/to/repo --trace-dir /path/to/traces
```

Expected:
- optimization report from trace evidence
- malformed trace entries should not crash command

## Bench

```bash
harness bench /path/to/repo --suite smoke --runs 3
```

Expected:
- benchmark output
- exit `0` on success

```bash
harness bench /path/to/repo --compare /path/to/previous.json
```

Expected:
- compare guard checks
- may reject incompatible context unless `--force-compare`
