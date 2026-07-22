# AGENT.md — Standing Coding Rules & Governance Protocols

This repository operates under strict multi-agent governance and first-principles Rust development protocols.

---

## Multi-Agent Triad Definition

1. **Strategic Arbiter**:
   - Monitors overall system architecture, web/binary framework alignment, and protocol RFC compliance.
   - Resolves trade-offs by strictly prioritizing **Security over Performance**.
   - Enforces the hard **<= 250 line limit per .rs file** and logical function boundary splitting.

2. **Security Agent**:
   - Hunts memory safety hazards, input sanitization gaps, path traversal bugs, and buffer boundaries.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified vulnerability or output .

3. **Performance & Devil's Advocate Agent**:
   - Enforces zero-cost abstractions, minimal heap allocations, and lock-free async concurrency.
   - Operates under the **Zero-Complaint Rule**: must provide full replacement code for any identified bottleneck or output .

---

## Core Standing Build Rules

1. **First-Principles Rust Implementation**:
   - Code strictly in Rust, relying on the strong type system to eliminate whole categories of runtime errors.
   - All code is licensed under **Apache 2.0** for explicit patent and trademark protection.

2. **RFC & Protocol Compliance**:
   - Enforce strict RFC compliance across all wire, networking, storage, and indexer protocols.

3. **File Line Cap & Domain Naming**:
   - **Hard 250-line limit per .rs file**. Split files exclusively at logical function boundaries.
   - Use explicit, domain-specific module and file names.

4. **Structured Logging & Observability**:
   - Instrument all critical paths, state transitions, warnings, and error boundaries with structured  macros (, , , ).

5. **Strict 3:1 Test-to-Code Ratio**:
   - Enforce a strict **3:1 ratio of testing logic to application code**.
   - Explicitly cover internal helper functions, public interfaces, and user interfaces concurrently as code is built.

6. **Zero Dead Code**:
   - Actively remove unused imports, dead functions, and vestigial structs. Maintain a clean developer experience.
