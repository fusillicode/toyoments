# toyoments

![Top language](https://img.shields.io/github/languages/top/fusillicode/toyoments)
[![CI](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml)
[![Doc](https://github.com/fusillicode/toyoments/actions/workflows/doc.yml/badge.svg)](https://fusillicode.github.io/toyoments)
[![Commits](https://shields.io/github/last-commit/fusillicode/toyoments)](https://github.com/fusillicode/toyoments/commits/main)

A small experimental payment engine to simulate processing of clients transactions.
It ingests a CSV of transaction, mutates in‑memory client accounts, and emits a final CSV report (one row per client) to stdout.

## Overview

- Input: CSV with columns `type,client,tx,amount`.
- Supported transaction types: `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`.
- Output: CSV with columns `client_id,available,held,total,locked` (sorted by `client_id`).
- Errors: errors (e.g. malformed row, business rule violations, etc.) are logged to stderr without breaking the processing of subsequent transactions (see [Assumptions](#assumptions) / [Future Improvements](#future-improvements)).

## Build & Run

```bash
cargo run -- path/to/transactions.csv > report.csv
```

Errors (if any) will be printed to stderr; redirect separately if needed:

```bash
cargo run -- transactions.csv > report.csv 2> errors.log
```

## Testing

Snapshot integration tests assert full stdout. To update snapshots:

```bash
INSTA_UPDATE=auto cargo test
```

## Input Format (Example)

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

Whitespaces from CSV fields and headers are automatically trimmed.
Negative amounts are rejected.

## Output Format (Example)

```csv
client_id,available,held,total,locked
1,4.0,0.0,4.0,false
2,3.0,0.0,3.0,true
```

## Assumptions

- Transactions in the input CSV are **already sequentially ordered per client**.
- No current error condition aborts the entire run. If that policy changes, the main loop must classify fatal errors and `exit(1)` early.
- Errors are **non‑blocking** and printed to stderr; processing of subsequent transactions continues.
- If per‑client fatal semantics become necessary, a strategy must be defined. Possible options:
    - Record only the first error for each account
    - Record multiple errors for each account
    - Decide whether to ignore, quarantine, or still apply later transactions in case of error(s)

### Dispute Semantics

Flag‑Only Dispute Handling with no provisional credit for withdrawals.
`chargeback` on a withdrawal locks the account without refund (i.e. not a consumer chargeback refund) to:

- Avoid artificial inflation of `total` if "holding" a withdrawal (which already reduced balances).
- Keep invariants simple with `total` never exceeding true economic value unless an explicit refund occurs.

Behaviour in case of deposits:

- Dispute: Move disputed amount from `available` to `held` (freeze spendability; `total` unchanged).
- Resolve: Release held funds back to `available` (state returns to pre‑dispute; no net effect).
- Chargeback: Permanently remove held funds and lock the account.

Behaviour in case of withdrawals:

- Dispute: no immediate balance change (no provisional refund or hold increase).
- Resolve: Refund (re‑credit) the withdrawn amount to `available` (customer win scenario).
- Chargeback: Lock account without refund (fraud/account risk lock). Withdrawal debit stands.

Re-dispute after resolve are allowed, permitting repeated dispute cycles.

## Error Handling (Current)

- CSV deserialization errors are logged to stderr and the processing of the related row skipped.
- Business rule errors (e.g. insufficient funds, invalid dispute context) are logged to stderr and the processing of the related transaction skipped.
- Reporting errors (e.g. overflow on `total` computation, failed serialization, I/O errors) are collected and logged to stderr.

## Design Notes

- Maintaining a `HashMap` for accounts yields amortized O(1) mutation 
- Ordering for a deterministic output is done by sorting once at output time.
- Decimal arithmetic uses `rust_decimal` to preserve fixed precision. Client account's `total` is computed with overflow checking.

## Limitations

- No persistence (in‑memory only).
- No concurrency / parallelism yet.
- Error verbosity can be noisy for large inputs.

## Future Improvements

- Implement a more refined dispute semantic to grant clients a better UX.
- If current dispute semantic in kept, rename withdrawal `chargeback` to `fraud_lock` and split `resolve` into explicit `customer_win` / `merchant_win`.
- Handle re-disputes by (a) forbidding them on the same transaction, or (b) track dispute life cycle.
- Optimize chargeback by pruning related transaction to reduce memory and forbid further life cycle actions.
- Introduce structured error policy (global fatal vs per‑client fatal vs recoverable) and clear exit codes.
- Simplify error payloads by using IDs rather than whole models
- Improve errors display representations and summary (e.g. [NDJSON](https://en.wikipedia.org/wiki/JSON_streaming#Newline-Delimited_JSON))
- Abstract account storage behind a trait (enables alternate backends / persistence).
- Explore an event‑sourced redesign: explicit aggregate state, events, and transitions.
- Parallelize per‑client processing by introducing Kafka (partition by client id + consumer group) or re‑design the solution following a dataflow programming approach (e.g. [Timely Dataflow](https://github.com/TimelyDataflow/timely-dataflow)).
- Consider batched or streaming snapshotting to external storage.
