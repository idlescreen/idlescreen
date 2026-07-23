# AGENT.md — Project Genesis & Build Rules (trance)

This repository operates under strict multi-agent governance and first-principles Rust development protocols.

---

## Multi-Agent Triad Definition

1. **Strategic Arbiter**:
   - Owns system architecture, module boundaries, phase gates, and RFC/protocol compliance.
   - Resolves trade-offs by strictly prioritizing **Security over Performance**.
   - Enforces the hard **<= 250 line limit per .rs file** and logical function boundary splitting.

2. **Security Agent**:
   - Hunts memory safety hazards, input sanitization gaps, path traversal bugs, and buffer boundaries.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified vulnerability or explicitly output `PASS: SECURITY AUDIT CLEAN`.

3. **Performance & Devil's Advocate Agent**:
   - Enforces zero-cost abstractions, minimal heap allocations, and lock-free async concurrency.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified bottleneck or explicitly output `PASS: PERFORMANCE AUDIT CLEAN`.

---

## Core Standing Build Rules

1. **First-Principles Rust Implementation**:
   - Code strictly from first principles in Rust, leveraging the strong type system to eliminate runtime failure states.
   - All code is licensed under the **Apache 2.0 License** for explicit patent protection.

2. **RFC & Protocol Compliance**:
   - Guarantee strict RFC compliance across all HTTP, DBus, and IPC protocols.

3. **File Line Cap & Domain Naming**:
   - **Hard 250-line limit per .rs file**. Split files exclusively at logical function boundaries.
   - Mandate explicit, domain-specific module, file, and symbol naming.

4. **Structured Logging & Observability**:
   - Embed structured logging (`tracing`) across all critical execution paths, error boundaries, and state transitions.

5. **Zero-Trust Dependency Rule**:
   - Default strictly to the Rust Standard Library (`std`) first.
   - Restrict third-party crates to highly vetted, industry-standard libraries.

6. **No `.unwrap()` or `.expect()` in Production Code**:
   - Strictly forbid the use of `.unwrap()` and `.expect()` outside of test modules (`#[cfg(test)]`). All production error cases must use proper `Result` propagation and pattern matching.

7. **3:1 Test-to-Code Ratio Goal**:
   - Pursue a **3:1 test-to-code ratio goal**—not a hard requirement, but a forcing function for defect discovery, regression shielding, and decoupled architecture.
   - Explicitly cover internal helper functions, public logic, and user interfaces concurrently as code is built.

8. **Continuous State Tracking**:
   - Every change made during any phase must be immediately committed and pushed to GitHub.

9. **Zero Dead Code**:
   - Actively eliminate unused imports, dead functions, and vestigial code paths.
