# toyoments

![Top language](https://img.shields.io/github/languages/top/fusillicode/toyoments)
[![CI](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml)
[![Doc](https://github.com/fusillicode/toyoments/actions/workflows/doc.yml/badge.svg)](https://fusillicode.github.io/toyoments)
[![Commits](https://shields.io/github/last-commit/fusillicode/toyoments)](https://github.com/fusillicode/toyoments/commits/main)

An experimental in‑memory payment engine: reads a CSV of transactions, applies them to client accounts, and writes a per‑client CSV report to stdout.

## Overview

- Input CSV header: `type,client,tx,amount` (amount omitted for `dispute|resolve|chargeback`)
- Transaction types: `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`
- Output columns: `client_id,available,held,total,locked` (sorted by `client_id`)
- Errors (parse + business + reporting) are logged to stderr; processing continues
- Whitespace trimmed; negative amounts rejected
- Decimal math uses `rust_decimal`; original scale preserved (no extra rounding)

### Input Schema

| Column   | Type            | Required    | Notes                                                          |
| -------- | --------------- | ----------- | -------------------------------------------------------------- |
| `type`   | string          | yes         | One of `deposit`,`withdrawal`,`dispute`,`resolve`,`chargeback` |
| `client` | unsigned 16-bit | yes         | `u16` client id                                                |
| `tx`     | unsigned 32-bit | yes         | `u32` transaction id                                           |
| `amount` | decimal         | conditional | Only for `deposit`,`withdrawal`; must be positive              |

### Output Schema

| Column      | Type            | Notes                                                                  |
| ----------- | --------------- | ---------------------------------------------------------------------- |
| `client_id` | unsigned 16-bit | Sorted ascending                                                       |
| `available` | decimal         | Spendable funds                                                        |
| `held`      | decimal         | Frozen funds (disputed deposits only)                                  |
| `total`     | decimal         | `available + held`; overflow checked; never negative                   |
| `locked`    | boolean         | True after terminal chargeback (deposit loss or withdrawal fraud lock) |

### Example Input

```csv
type,client,tx,amount
deposit,1,1,5.1234
deposit,2,3,3.0000
dispute,1,1,
withdrawal,2,4,2.0000
resolve,1,1,
withdrawal,1,2,1.1234
dispute,2,4,
chargeback,2,4,
```

### Example Output

```csv
client_id,available,held,total,locked
1,4.0,0.0,4.0,false
2,1.0,0.0,1.0,true
```

## Build & Run

```bash
cargo run -- path/to/transactions.csv > report.csv     # stdout: report
cargo run -- transactions.csv > report.csv 2> errors.log
```

## Testing

Snapshot integration tests assert full stdout. To update snapshots:

```bash
INSTA_UPDATE=auto cargo test
```

## Assumptions

- Input transactions are already sequentially ordered per client.
- No current error condition aborts the entire run; all errors are non‑fatal.

### Dispute Semantics

#### Deposits:

- Dispute: move amount `available → held` (total unchanged)
- Resolve: release `held → available`
- Chargeback: remove held funds; lock account

#### Withdrawals (asymmetric “flag-only” model):

- Dispute: mark disputed; balances unchanged
- Resolve: single refund (first successful resolve only) → credit back to `available`
- Chargeback: lock account; no refund (debit stands)

#### Invariants & Rationale:

- Avoids “phantom asset” inflation from disputing already-debited withdrawals
- `total` reflects realised net unless a refund is explicitly granted
- Single-refund invariant prevents repeated credit cycling
- Re-disputes allowed; further resolves on same withdrawal are idempotent
- Withdrawal chargeback currently means fraud/risk lock (future rename e.g. `fraud_lock`)

## Error Model

- Parse errors: transaction processing skipped, error logged
- Business rule errors (e.g. insufficient funds, invalid dispute context): transaction processing skipped, error logged
- Reporting errors (serialization / overflow / I/O): collected and logged
- None are fatal to the overall run (future policy could classify fatal vs recoverable)

## Design Notes

- Accounts stored in `HashMap` for amortized O(1) inserts/updates; final report performs a one‑shot sort (O(n log n)) for deterministic ordering & snapshot testing.
- Alternative: `BTreeMap` (inherent ordering at O(log n) per mutation) — omitted to favour cheaper hot‑path mutations.
- `total` computed with overflow guard.

## Limitations

- No persistence, in‑memory only
- No concurrency / parallelism support
- Verbose stderr on large inputs

## Future Improvements

- Implement a more refined withdrawal dispute semantic (e.g. provisional refunds) for better client UX.
- If current semantic is kept, consider renaming withdrawal `chargeback` to `fraud_lock` and splitting `resolve` into explicit `customer_win` / `merchant_win`.
- Handle re‑disputes explicitly: forbid on the same transaction OR track full dispute life cycle.
- Optimize `chargeback` by pruning related transactions to reduce memory AND forbid further life cycle actions.
- Introduce structured error policy:
  - Classify whole‑process fatal, per‑client fatal (quarantine), recoverable.
  - Evaluate per‑client error strategies:
    - Record only the first error per account
    - Record multiple errors per account
    - Decide whether later transactions are ignored, quarantined, or still applied after an error
- Simplify error payloads (use IDs instead of full structs).
- Improve error output formats (e.g. [NDJSON](https://en.wikipedia.org/wiki/JSON_streaming#Newline-delimited_JSON) stream).
- Abstract account storage behind a trait to enable swapping/mocking persistence.
- Explore an Event Sourced redesign with dedicated aggregates, states, events, and transitions.
- Parallelize per‑client processing via, e.g. Kafka partitioned topic by client id + consumer group, or switching to a dataflow programming approach (https://github.com/TimelyDataflow/timely-dataflow).
