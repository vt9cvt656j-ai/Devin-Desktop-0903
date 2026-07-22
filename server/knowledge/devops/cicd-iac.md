# CI/CD & Infrastructure as Code

## CI/CD Pipeline Patterns

### GitHub Actions Structure
```yaml
name: CI
on:
  push: { branches: [main] }
  pull_request: { branches: [main] }

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: npm }
      - run: npm ci
      - run: npm test
      - run: npm run lint
      - run: npm run build

  deploy:
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # ... deploy steps
```

### Pipeline Self-Repair
On failure:
1. Capture job logs, truncate to fit context
2. Classify: syntax fix (auto-apply + re-run) | infra issue (rollback) | ambiguous (escalate to human)
3. Store successful auto-fixes as few-shot examples for future failures

### Pipeline Rules
- Cache dependencies (`actions/cache` or built-in cache)
- Run tests in parallel where possible
- Fail fast: lint → type-check → unit test → integration test → build → deploy
- Secrets: never hardcode, use `${{ secrets.X }}`
- Branch protection: require passing CI + review before merge

## Infrastructure as Code

### Terraform Best Practices
```hcl
# Module structure
modules/
  vpc/
  compute/
  database/
environments/
  dev/
  staging/
  prod/
```

### Validation Loop (Critical — 42.7% of syntactically correct IaC fails on deploy)
```
1. Generate Terraform/CloudFormation
2. terraform validate (syntax)
3. terraform plan (semantic simulation)
4. OPA/Rego policy check (security compliance)
5. If any fail → extract structured error → feed back → fix → repeat (up to 5 rounds)
6. Only deploy after all 3 pass
```

### Terraform Rules
- State in remote backend (S3 + DynamoDB lock)
- Pin provider versions
- Use `count`/`for_each` for repetitive resources
- Tag everything: `Name`, `Environment`, `Team`, `CostCenter`
- Never use `terraform destroy` without explicit confirmation
- Modularize: reusable modules for VPC, compute, database, IAM

### Security (Only 8.4% of generated IaC passes security checks without enforcement)
- Run Checkov/tfsec before apply
- Enforce: encryption at rest, no public S3 buckets, no 0.0.0.0/0 security groups
- IAM: least privilege, no inline policies, no wildcard actions

## Monitoring & Alerting

### Alert Template
```yaml
alert:
  name: "High API Latency"
  condition: "p99_latency > 1200ms for 5 minutes"
  current_value: "{{ value }}"
  baseline: "200ms"
  severity: "critical"
  runbook_url: "https://wiki/runbooks/high-latency"
  diagnostic_steps:
    - "Check recent deployments"
    - "Check database query times"
    - "Check upstream service health"
```

### SLO Targets
- API availability: > 99.9%
- P99 latency: < 1200ms
- Error rate: < 0.1%
- Tool call success: > 99%

### Stack
- Metrics: Prometheus (time-series) + Grafana (dashboards)
- Logs: structured JSON → OpenTelemetry → centralized store
- Traces: OpenTelemetry SDK, W3C Trace Context propagation
- Alerts: severity-based routing (critical → PagerDuty, warning → Slack)
