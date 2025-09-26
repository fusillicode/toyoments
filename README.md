# toyoments

![Top language](https://img.shields.io/github/languages/top/fusillicode/toyoments)
[![CI](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/fusillicode/toyoments/actions/workflows/ci.yml)
[![Doc](https://github.com/fusillicode/toyoments/actions/workflows/doc.yml/badge.svg)](https://fusillicode.github.io/toyoments)
[![Commits](https://shields.io/github/last-commit/fusillicode/toyoments)](https://github.com/fusillicode/toyoments/commits/main)

A small experimental payment engine. It ingests a CSV of transaction, mutates in‑memory client accounts, and emits a final CSV report (one row per client) to stdout.

## Overview

- Input: CSV with columns `type,client,tx,amount`.
- Supported transaction types: `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback`.
- Output: CSV with columns `client_id,available,held,total,locked` (sorted by `client_id`).
- Errors: Non‑fatal issues (e.g. malformed row, business rule violation) are logged to stderr and processing continues (see Assumptions / Future Improvements).

## Build & Run

```bash
cargo run -- path/to/transactions.csv > report.csv
```

Errors (if any) will be printed to stderr; redirect separately if needed:

```bash
cargo run -- transactions.csv > report.csv 2> errors.log
```

## Testing

Snapshot tests assert full stdout for determinism. Update snapshots:

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
- If per‑client fatal semantics become necessary, a strategy is still TBD (e.g. record first error on the account; decide whether to ignore, quarantine, or still apply later transactions).

### Dispute Semantics

Flag‑Only Dispute Handling (no provisional credit for withdrawals). 
`chargeback` on a withdrawal locks the account without refund (i.e. not a consumer chargeback refund) to:

- Avoid artificial inflation of `total` if "holding" a withdrawal (which already reduced balances).
- Keep invariants simple with `total` never exceeding true economic value unless an explicit refund occurs.

Deposits:

- dispute: Move disputed amount from `available` to `held` (freeze spendability; `total` unchanged).
- resolve: Release held funds back to `available` (state returns to pre‑dispute; no net effect).
- chargeback: Permanently remove held funds and lock the account.

Withdrawals:

- dispute: no immediate balance change (no provisional refund or hold increase).
- resolve: Refund (re‑credit) the withdrawn amount to `available` (customer win scenario).
- chargeback: Lock account without refund (fraud/account risk lock). Withdrawal debit stands.


## Error Handling (Current)

- Deserialization failures: logged, row skipped.
- Business rule errors (e.g. insufficient funds, invalid dispute context): logged, transaction skipped.
- Overflow on `total` computation: reported via error propagation (would surface as a runtime error if it occurred).

## Design Notes

- Maintaining a `HashMap` for accounts yields amortized O(1) mutation; final deterministic ordering achieved by sorting once at output time.
- Decimal arithmetic uses `rust_decimal` to preserve fixed precision; total is computed with overflow checking.

## Limitations

- No persistence (in‑memory only).
- No concurrency / parallelism yet.
- Error verbosity can be noisy in large inputs.

## Future Improvements

- Rename withdrawal `chargeback` path to `fraud_lock` and split `resolve` into explicit `customer_win` / `merchant_win` decisions.
- Introduce structured error policy (global fatal vs per‑client fatal vs recoverable) and clear exit codes.
- Slim down error payloads (prefer stable IDs) and improve human‑readable formatting.
- Abstract account storage behind a trait (enables alternate backends / persistence).
- Explore an event‑sourced redesign: explicit aggregate state, events, and transitions.
- Parallelize per‑client processing: e.g. Kafka (partition by client id + consumer group) or a dataflow style pipeline.
- Consider batched or streaming snapshotting to external storage.
