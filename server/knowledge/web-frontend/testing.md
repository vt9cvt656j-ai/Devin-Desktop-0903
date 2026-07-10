# Frontend & Code Testing Patterns

## Test Generation Strategy

### Unit Tests — Mutation-Guided Approach (Meta ACH: 73% acceptance rate)
```
1. Given a function, generate 3-5 realistic single-line mutations (plausible bugs)
2. For each mutant: "Write a test that passes on original but fails on this mutant"
3. Filter equivalent mutants: "Is this mutant semantically equivalent? If yes, skip"
4. Run tests: keep only those that compile + pass on original + kill ≥1 mutant
5. Adversarial round: generate mutants that survive existing tests → write tests to catch them
6. Repeat 3-5 rounds (diminishing returns after 5)
```

### Coverage-Guided Iteration (CoverUp: median 80% coverage)
```
1. Generate initial tests
2. Run coverage analysis → identify uncovered lines
3. Prompt: "Lines 4-7, 12-15 do not execute. Write tests targeting those paths."
4. If tests fail: extract error → "This test yields [error]. Fix it."
5. If tests pass but no new coverage: point out remaining uncovered lines
6. Repeat up to 3 iterations (60% success from 1st prompt, 40% from iterations)
7. Run each test in isolation to prevent state pollution
8. Run each new test multiple times to detect flakiness
```

## E2E Testing — Playwright

### Three-Level Decomposition (GenIA: 82% precision)
```
Level 1: Segment test scenarios by URL transitions → JSON modules
Level 2: Crawl page HTML → extract XPath selectors → validate uniqueness
Level 3: Generate executable test scripts with proper waits/assertions
```

### Playwright Agents (Built-in since v1.56)
- **Planner**: explores live app → Markdown test plan
- **Generator**: plan → TypeScript test files
- **Healer**: executes suite → auto-repairs broken selectors

### E2E Rules
- Test the golden path first, then edge cases
- Use `getByRole()` / `getByText()` over CSS selectors (more resilient)
- Explicit waits: `await page.waitForSelector()` not `sleep()`
- Screenshot at 3 viewports: 375px, 768px, 1440px
- Self-healing resolves ~65% of broken selectors; 35% need human review

## Component Testing (React)
```javascript
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

test('submits form with valid data', async () => {
  const onSubmit = vi.fn()
  render(<Form onSubmit={onSubmit} />)

  await userEvent.type(screen.getByLabelText('Email'), 'test@example.com')
  await userEvent.click(screen.getByRole('button', { name: 'Submit' }))

  expect(onSubmit).toHaveBeenCalledWith({ email: 'test@example.com' })
})
```

Rules:
- Query by role/label, not test IDs or class names
- `userEvent` over `fireEvent` (more realistic)
- Test behavior, not implementation details
- Mock at the network boundary (`msw`), not internal modules

## Refactoring Validation
```
1. Generate random/edge-case inputs for affected functions (10-50)
2. Run both original and refactored code against inputs
3. Compare outputs — any divergence = semantic change → reject
4. For functions with side effects: compare observable state changes
5. Only accept refactorings passing all differential tests
```

## Code Review Checklist (Automated)
```
1. Detect: security, logic errors, performance, style issues
2. Localize: exact file path, line range, code snippet
3. Prioritize: security > correctness > performance > style
4. Multi-round: generate 3 reviews, keep findings in 2+ of them
5. Cross-validate with static analysis (ESLint/SonarQube)
```

## Technical Debt Detection
```
Scan for: TODO, FIXME, HACK, XXX, WORKAROUND
+ Natural language: "temporary", "should be refactored", "workaround for"
Classify: single-line fix | multi-line fix | architectural change
Auto-fix: single-line and multi-line only
Architectural: generate remediation plan, flag for human review
```
