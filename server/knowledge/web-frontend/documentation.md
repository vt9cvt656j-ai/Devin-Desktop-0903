# Code Documentation & Refactoring

## Documentation Generation (DocAgent: 95.74% truthfulness)

### Five-Agent Pipeline
```
1. Dependency Analyzer: build topological order of modules
2. Reader: extract signatures, types, relationships per module
3. Writer: generate documentation following module dependency order
   (document dependencies BEFORE dependents — eliminates hallucination from missing context)
4. Verifier: cross-check generated docs against source code
5. Reviewer: evaluate completeness, accuracy, style consistency
```

### Documentation Rules
- Every public function: one-line summary + params + return type + example
- Never describe WHAT (code already does that) — describe WHY and edge cases
- Include usage examples that compile and run
- Update docs when modifying the function — stale docs are worse than none
- API docs: include request/response examples with realistic data

## Refactoring Strategy (Conservative — aggressive agents introduce 140 new smells fixing 31)

### Before Refactoring
```
1. Run full test suite — all green = baseline
2. Generate 10-50 edge-case inputs for affected functions
3. Record outputs for ALL inputs (golden test set)
4. Identify the specific smell to fix (don't fix neighbors)
```

### During Refactoring
```
1. Make ONE type of change at a time (extract method, rename, inline, etc.)
2. After each change: re-run tests + compare golden outputs
3. Any divergence = semantic change → revert that step
4. Maximum 3 extract-method refactorings per session
5. NEVER change behavior while refactoring structure
```

### After Refactoring
```
1. Run full test suite
2. Run golden test set comparison
3. Run linter/formatter
4. Diff review: confirm only structural changes, no behavioral changes
5. If tests fail: revert to last passing state, do NOT fix forward
```

### Code Smell Priority (Fix in This Order)
1. Long method (> 50 lines) → extract method
2. Duplicated code (3+ occurrences) → extract shared function
3. Large class (> 500 lines) → extract class
4. Long parameter list (> 4 params) → introduce parameter object
5. Feature envy → move method to the class it envies
6. Dead code → remove (verify with `grep -r` first)

### Anti-Patterns (NEVER Do)
- Don't refactor and add features in the same commit
- Don't "improve" code you're just passing through
- Don't introduce abstractions for 2 call sites (wait for 3)
- Don't refactor without tests (add tests first, then refactor)
- Don't rename across the entire codebase in one go (do module by module)

## Code Migration at Scale (Google: 50% time reduction)

### Large Codebase Migration Steps
```
1. Automated discovery: find all instances of pattern to migrate
2. Classify instances by complexity (simple/medium/complex)
3. Auto-migrate simple instances (mechanical transforms)
4. Generate migration suggestions for medium instances
5. Flag complex instances for human review
6. Verify ALL migrations pass existing tests
7. Run migration in batches, not all-at-once
```
