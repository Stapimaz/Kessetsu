# ADR 0004: Evaluator-Owned Requirement Sets

- Status: accepted
- Date: 2026-09-15
- Scope: Core assertion compilation, native CLI, agent workflows, and structured test output

## Context

Inline assertions are convenient when one person owns both a circuit and its tests. They are not a sufficient trust boundary when an AI agent is allowed to revise the circuit: the same agent could weaken or remove the assertions and still present a successful `kess test` result. The independent Phase 4 evaluators avoided this problem with task-specific harnesses, but the public CLI needs a general, low-friction equivalent.

The solution must keep assertion semantics in Rust Core, preserve existing `.kess` files, avoid a second measurement language, and report exactly which external requirements were evaluated. It cannot claim that a normal writable file is cryptographically immutable; ownership and filesystem permissions remain the caller's responsibility.

## Decision

Kessetsu will support evaluator-owned `.kessreq` files through:

```text
kess test design.kess --requirements amplifier.kessreq
kess test design.kess --requirements amplifier.kessreq --requirements-sha256 <64-hex-digest>
```

A `.kessreq` file is an assertion-only subset of the existing language. Comments and one or more ordinary `assert ...` statements are allowed; component declarations, models, modules, connections, nets, analyses, and raw simulator directives are not. The format is identified by the versioned `kessetsu.requirements.v1` Core contract rather than a second parser or duplicated expression syntax.

### Authority and composition

- Without `--requirements`, existing inline `.kess` assertions behave exactly as before.
- With `--requirements`, the design must contain no inline assertions. Mixed ownership fails closed rather than silently merging, replacing, or prioritizing two requirement sources.
- Core compiles the external assertions into the same typed `Assertion` objects used in `CircuitIR`. The CLI attaches them to the successfully compiled design IR before simulation and evaluation. Measurement execution never parses the requirements file directly.
- An empty requirement set fails before simulator discovery. Unsupported metrics, invalid argument shapes, and dimensional errors use the existing semantic validation.

### Identity and pinning

Core computes SHA-256 over the exact `.kessreq` bytes and returns it with the schema version and assertion count. Structured CLI output records this metadata for every valid external requirement run and never includes an absolute requirements path.

`--requirements-sha256` optionally pins the caller's expected digest. It accepts a 64-character hexadecimal digest, with or without the `sha256:` prefix, and fails before simulation on malformed input or mismatch. Recording a digest proves which bytes were evaluated; it does not prove who authored the file. A supervising agent, CI job, or user must keep the requirement file or expected digest outside the design agent's write authority when adversarial modification matters.

### CLI and Web boundary

The first contract is native-CLI only because it exists primarily for supervised agent and CI loops. It adds no account, upload, or server dependency. Browser support may later bind a user-selected requirement file to the same Core contract, but the first Web release continues to use assertions embedded in its local project source.

The `test` JSON envelope exposes the requirement schema, exact hash, assertion count, and whether an expected hash was supplied. Invalid requirement input is a structured diagnostic and cannot produce a successful test status. `simulate` remains the correct command when no assertions are intended.

## Consequences

- A design agent can revise `design.kess` repeatedly while a supervisor holds `requirements.kessreq` fixed.
- Existing inline-assertion workflows and all current `.kess` examples remain compatible.
- There is one assertion language and one measurement implementation, not an evaluator-specific fork.
- Users who do not isolate or pin the external file still gain provenance and separation, but not an absolute tamper-proof guarantee.
- Extending `.kessreq` beyond assertions or combining requirement sources requires a new ADR and version review.

## Acceptance evidence

Core tests must cover exact-byte hashing, assertion-only validation, empty files, invalid metrics, and deterministic typed output. CLI tests must prove successful and failing external evaluation, hash pin success/mismatch, rejection of mixed inline/external authority, no simulator launch on invalid input, structured provenance, and unchanged inline assertion behavior.
