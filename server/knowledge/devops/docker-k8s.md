# Docker & Kubernetes Best Practices

## Dockerfile Rules

```dockerfile
# Multi-stage build (always)
FROM node:22-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci --production=false
COPY . .
RUN npm run build

FROM node:22-alpine AS production
WORKDIR /app
COPY --from=build /app/dist ./dist
COPY --from=build /app/node_modules ./node_modules
USER node
EXPOSE 3000
CMD ["node", "dist/index.js"]
```

Checklist:
- Multi-stage builds — separate build deps from runtime
- `.dockerignore` excludes: `node_modules`, `.git`, `*.md`, `.env`
- Run as non-root user (`USER node` / `USER 1000`)
- Pin base image versions (`node:22.5-alpine`, not `node:latest`)
- Order layers by change frequency: OS deps → app deps → source code
- Use `COPY` not `ADD` (unless extracting tar)
- One process per container
- Health check: `HEALTHCHECK CMD curl -f http://localhost:3000/health || exit 1`

## Kubernetes Security Hardening

### Pod Security
```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop: ["ALL"]
```

### Resource Limits (always set)
```yaml
resources:
  requests:
    cpu: "100m"
    memory: "128Mi"
  limits:
    cpu: "500m"
    memory: "512Mi"
```

### Network Policies
- Default deny all ingress/egress
- Explicitly allow only needed communication paths
- Generate policies from observed traffic (Hubble/Cilium), not guesswork

### Probes
```yaml
livenessProbe:
  httpGet: { path: /health, port: 3000 }
  initialDelaySeconds: 15
  periodSeconds: 20
readinessProbe:
  httpGet: { path: /ready, port: 3000 }
  initialDelaySeconds: 5
  periodSeconds: 10
```

## Kubernetes Troubleshooting

When diagnosing failures, use dual-process routing:
1. **Fast path**: if symptoms match a known pattern (OOMKilled, CrashLoopBackOff, ImagePullBackOff) → apply known fix
2. **Slow path**: if unfamiliar → gather logs + describe + events → reason through causal chain

Common failures:
| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| CrashLoopBackOff | App crashes at startup | Check logs, fix app config |
| OOMKilled | Memory limit too low | Increase limit, fix memory leak |
| ImagePullBackOff | Wrong image name/tag | Fix image reference, check registry auth |
| Pending | No schedulable node | Check node resources, pod affinity |
| Evicted | Node under pressure | Increase node resources, set priority |

## Helm Chart Patterns
- Use `values.yaml` for all configurable values
- Template helpers in `_helpers.tpl` for DRY
- Always include NOTES.txt for post-install instructions
- Pin chart versions in `Chart.lock`
