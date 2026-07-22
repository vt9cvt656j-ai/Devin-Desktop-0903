# Rust Development Patterns

## Compiler-in-the-Loop Iteration (93.5% compilation repair rate)

```
1. Generate initial Rust code
2. cargo build → capture ALL error messages
3. Feed error messages (including error codes like E0382) + source to LLM
4. Fix specifically the reported errors
5. Repeat until compilation succeeds or budget exhausted (max 5 rounds)
6. cargo test for semantic verification
7. cargo clippy for idiomatic improvements
```

## Five Anti-Patterns to Always Prevent

| Anti-Pattern | Rule |
|-------------|------|
| Unnecessary `unsafe` | Never use `unsafe` unless no safe alternative exists. Always explain why |
| Blocking in async | Never call `std::fs`/`std::net` in async. Use `tokio::fs`/`tokio::net` |
| MutexGuard across await | Never hold `std::sync::MutexGuard` across `.await`. Clone data or use `tokio::sync::Mutex` |
| Missing FFI null checks | At FFI boundaries, always check pointers for null before dereferencing |
| Unwrap in production | Use `?` operator, not `.unwrap()` — except in tests and examples |

## Ownership & Borrowing Quick Reference

```rust
// Prefer borrowing over cloning
fn process(data: &[u8]) -> Result<()> { ... }  // Good: borrows
fn process(data: Vec<u8>) -> Result<()> { ... } // Bad: takes ownership unnecessarily

// Use Cow for conditional ownership
fn normalize(s: &str) -> Cow<'_, str> {
    if s.contains('\n') { Cow::Owned(s.replace('\n', " ")) }
    else { Cow::Borrowed(s) }
}

// Builder pattern for complex structs
impl Config {
    fn builder() -> ConfigBuilder { ConfigBuilder::default() }
}
```

## Async Concurrency (Tokio)

```rust
// Structured concurrency with JoinSet
let mut set = JoinSet::new();
for item in items {
    set.spawn(async move { process(item).await });
}
while let Some(result) = set.join_next().await {
    handle(result??);
}

// Graceful shutdown with CancellationToken
let token = CancellationToken::new();
tokio::select! {
    _ = token.cancelled() => { /* cleanup */ }
    result = do_work() => { handle(result); }
}

// spawn_blocking for CPU-intensive work
let result = tokio::task::spawn_blocking(|| expensive_computation()).await?;
```

## Error Handling

```rust
// Use thiserror for library errors
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

// Use anyhow for application errors
fn main() -> anyhow::Result<()> {
    let config = load_config().context("failed to load config")?;
    Ok(())
}
```

## C-to-Rust Migration (Skeleton-First)

```
1. Extract C codebase function signatures, types, call graph
2. Generate Rust skeleton: struct definitions, fn signatures, trait bounds
3. Fill implementations function-by-function using C source as reference
4. Replace unsafe C lock APIs with Mutex/RwLock
5. Iterative compile-test-verify per function
6. Measured: 24% improvement in safety of lines, 37% end-to-end success
```

## Performance Optimization

### GPU Kernel Optimization (CUDAMaster: up to 1.8x over cuDNN)
```
1. Profile kernel with NVIDIA Nsight → classify bottleneck
   - Compute-bound → optimize ALU utilization, instruction-level parallelism
   - Memory-latency-bound → prefetching, occupancy tuning
   - Memory-bandwidth-bound → coalescing, shared memory tiling
2. Apply bottleneck-specific optimization
3. Validate correctness AND performance
4. Iterate with profiling feedback in natural language
```

### General Performance Rules
- Profile before optimizing (criterion for benchmarks)
- Prefer `Vec` over `LinkedList` (cache locality)
- Use `SmallVec` for usually-small collections
- `rayon` for easy data parallelism
- Avoid allocations in hot paths — preallocate and reuse
