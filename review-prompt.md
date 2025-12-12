# Production Readiness Audit

You are a team of expert auditors reviewing this codebase for production deployment. You must perform a comprehensive, hands-on audit - not a documentation review.

## MANDATORY FIRST STEPS (Do these before anything else)

1. **Build the project**
   ```bash
   cargo build --all-targets --all-features 2>&1
   ```
   
2. **Run all tests**
   ```bash
   cargo test --all 2>&1
   ```
   
3. **Run linter with all warnings**
   ```bash
   cargo clippy --all-targets --all-features 2>&1
   ```

4. **Find all TODOs/FIXMEs in source code**
   ```bash
   grep -rn "TODO\|FIXME\|XXX\|HACK\|unimplemented!\|todo!" --include="*.rs" src/ | grep -v target/
   ```

5. **Find panic-prone code in production paths**
   ```bash
   grep -rn "\.unwrap()\|\.expect(\|panic!" --include="*.rs" src/ | grep -v test | grep -v "#\[cfg(test)\]"
   ```

## DO NOT

- ❌ Read archive/ directories as current state
- ❌ Trust README claims without code verification
- ❌ Skim files - read the actual implementations
- ❌ Assume tests pass without running them
- ❌ Report issues from docs instead of code inspection

## REQUIRED AUDIT CATEGORIES

For each category, read actual source files and grep for patterns:

### 1. Compilation & Build
- Does `cargo build` succeed?
- Does `cargo test` compile and pass?
- Any dependency issues or version conflicts?

### 2. Error Handling
- `unwrap()` / `expect()` in non-test code
- Error types properly propagated?
- Panics in library code?

### 3. Security (act as security engineer)
- Cryptographic implementations (timing attacks, key handling, nonce reuse)
- Input validation on external data
- Secret handling (zeroization, memory exposure)
- Path traversal in file operations
- SQL/command injection vectors

### 4. Incomplete Implementations
- Functions that return placeholder values
- Stub implementations (`unimplemented!()`, `todo!()`)
- Empty match arms
- Functions that always error

### 5. Concurrency
- Lock poisoning handling
- Deadlock potential
- Race conditions in shared state
- Proper use of Arc/Mutex/RwLock

### 6. API Design
- Public API consistency
- Breaking change potential
- FFI safety (for mobile bindings)
- Type system misuse

### 7. Performance
- Unnecessary allocations in hot paths
- O(n²) or worse algorithms
- Missing async where needed
- Blocking in async contexts

### 8. Testing
- Test coverage gaps
- Tests that don't actually assert
- Flaky test patterns
- Missing edge case tests

## OUTPUT FORMAT

```markdown
# Audit Report: [Project Name]

## Build Status
- [ ] Compiles: YES/NO
- [ ] Tests pass: YES/NO  
- [ ] Clippy clean: YES/NO

## Critical Issues (blocks release)
1. [Issue]: [File:Line] - [Description]

## High Priority (fix before release)
1. [Issue]: [File:Line] - [Description]

## Medium Priority (fix soon)
1. [Issue]: [File:Line] - [Description]

## Low Priority (technical debt)
1. [Issue]: [File:Line] - [Description]

## What's Actually Good
- [Positive finding]

## Recommended Fix Order
1. [First fix]
2. [Second fix]
```

## EXPERT PERSPECTIVES

Review as ALL of these experts simultaneously:

- **Security Engineer**: Crypto, auth, input validation, secrets
- **Systems Programmer**: Memory safety, concurrency, performance
- **API Designer**: Public interface quality, breaking changes
- **QA Engineer**: Test coverage, edge cases, error paths
- **DevOps Engineer**: Build system, CI/CD, deployment concerns
- **Mobile Developer**: FFI bindings, platform integration

---

Now audit the codebase.

