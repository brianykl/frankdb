# frankdb Roadmap

A storage-disaggregated key-value database in Rust, inspired by
[Replay-as-a-Service (RaaS)](https://cs.purdue.edu/homes/csjgwang/pubs/SIGMOD26_RaaS.pdf)
(Pang & Wang, SIGMOD 2026).

**What this is:** a new Rust database and a simplified experiment inspired by the paper.
**What this is not:** a modification of PostgreSQL/OpenAurora or a faithful reproduction of the paper's evaluation.

## Current status

- **Current milestone:** 1, Vocabulary, reference model, deterministic workloads
- **Done:** workspace set up (`db`, `wal`, `replay`, `server`, `storage`); `wal` has a binary; build and tests pass; Rust Book ch. 4 read and quizzed
- **Next action:** read Rust Book ch. 5–8, answer the Milestone 1 pre-work questions, then implement
- **Known gaps to revisit:** why the borrow rules prevent dangling pointers (Vec reallocation); moves don't deep-copy; `&str`/`&[u8]` for read-only parameters

Update this section whenever a checkpoint passes.

---

## The research thread

The paper (verified sections):

- **§2.3 GetPage@LSN.** A reader asks storage for a page *as of* a log sequence number. Storage replays any unapplied log records for that page on demand.
- **§2.4 Background log replaying.** Storage keeps a per-page chain of unapplied log records. Background replay shortens those chains.
- **§3 Performance characterization.** Tail latency is driven by (1) how many log records a read must replay and (2) background replay competing with foreground reads for the storage node's limited CPU.
- **§5.1 Decoupling log replay logic.** Only the expensive replay step is moved to the replay service. The cheap "is replay needed?" metadata check stays on storage.
- **§5.3 Parallel replay.** Log records are sorted and grouped by page. Threads apply different pages independently.
- **§5.4 Control coordinator.** Replay tasks are scheduled by priority and by machine utilization.
- **§5.5 Fault tolerance.** Failed tasks are retried, dead workers are detected by heartbeat, and the coordinator is stateless.
- **Results:** p95 −40.1%, throughput +75.9% (SysBench, §7).

**The design tension we will measure.** In our first design, a read with a freshness floor *waits* until replay reaches that sequence number. In the paper, storage can *replay on demand* during the read, and background replay is only an optimization. In our first design, replay lag turns directly into read timeouts. Milestone 8 builds the paper's alternative so we can compare both.

---

## Milestones

Each milestone ends with a checkpoint: a test or demo that must pass before moving on.

| # | Milestone | Rust concepts | DB concepts | Checkpoint |
|---|---|---|---|---|
| 1 | Workspace, core types, reference model, deterministic workloads | cargo, modules, enums, ownership, `Result`, unit tests | operations, sequence numbers, defining correctness | Seeded random workloads match the reference model; same seed gives the same workload |
| 2 | Durable append-only log (library, single process) | `std::fs`, byte encoding, custom errors | write-ahead logging, fsync, framing, checksums, torn writes vs corruption | Truncation and bit-flip tests: recovery keeps every complete record and reports corruption |
| 3 | Storage with an applied position that survives crashes | traits, generics | checkpoints, atomic rename, safe repeated replay | A simulated crash at any point leaves state and applied position consistent |
| 4 | Replay (in process) | iterators, borrowing across components | ordered replay, safe duplicate replay | Replaying twice, or crashing halfway and resuming, gives the same result as the model |
| 5 | Split into 4 processes: server, log, replay, storage | async with `tokio`, TCP, message framing | service boundaries, retries, partial failure | **First end-to-end milestone:** write → log → replay → storage → read |
| 6 | Freshness-floor reads + the stop-replay demo | timeouts, channels, `select!` | freshness vs availability | With replay stopped, reads wait or time out; after restart, replay catches up and reads succeed |
| 7 | Benchmark harness + baseline | latency histograms, threads | p50/p95/p99, backlog, sustained vs burst | Sustained-load results with a bounded backlog, written up |
| 8 | **Frontier:** GetPage@LSN analogue | `Arc`, locks, parallelism | per-key log chains; on-demand vs background vs offloaded replay | Same workload with the technique on vs off: does p99 track chain length as in §3? |
| 9 | Parallel offloaded replay + scheduler | thread pools, work partitioning | §5.3 / §5.4 in miniature | Tail latency and CPU use vs number of replay workers |
| 10 | Demo app: event-offer tracker | CLI or HTTP | end-to-end use | Live demo: write prices, read with a freshness floor, kill replay, watch catch-up |

**Later:** a page-based storage engine (B-tree vs LSM, chosen after studying both), scans and secondary indexes, YCSB-style workloads, and a small PostgreSQL wire-protocol subset.

---

## Rust command cheat sheet

```sh
cargo new --lib <name>        # library crate
cargo new <name>              # binary crate (has main.rs)
cargo check                   # fast type-check
cargo build                   # compile (add --release for benchmarks)
cargo run -p <crate>          # run a binary in the workspace
cargo test                    # run all tests
cargo test <name> -- --nocapture   # run matching tests, show println! output
cargo clippy                  # linter
cargo fmt                     # format (cargo fmt --check in CI)
```

A workspace is one root `Cargo.toml` with `[workspace] members = [...]`, containing a shared library crate plus one binary crate per service. See *The Rust Programming Language* ch. 14.3.

---

## Milestone 1: Vocabulary, a reference model, and deterministic workloads

### Problem and why it matters
Before adding disk or network code, define the operations precisely and build a trivially correct reference model. Every later checkpoint (crashes, replay, multiple processes) is tested by running the same seeded workload through the real system and the model, then comparing results.

### Reading
- *The Rust Programming Language*: ch. 3–4 (ownership), 5–6 (structs, enums, `match`), 7 (modules), 9 (errors), 11 (tests), 14.3 (workspaces).
- RaaS §2.2–2.3 (write/read path, GetPage@LSN).

### Required behavior
- A cargo workspace with a core library crate.
- An operation type for put(key, value), get(key), and delete(key).
- A sequence-number type.
- A reference model that applies writes in order and answers gets.
- A workload generator: seed + parameters (operation count, key-space size, read/write/delete mix) → the same operation sequence every time.
- Errors appear in function signatures. No `panic!`/`unwrap` on conditions a caller could cause.

### Invariants
- The same seed always produces the same workload.
- After applying writes 1..n in order, the model's state is completely determined by those writes.
- Sequence numbers only ever increase.

### Design decisions to make and explain
- Keys and values: `String` or `Vec<u8>`? What does each cost when you write bytes to disk later?
- Sequence numbers: a raw `u64` or a wrapper type? Is 0 a valid position or a "nothing applied yet" marker?
- Delete semantics: should a get after a delete be distinguishable from a key that never existed? (Tombstones matter for replay later.)
- Randomness: the `rand` crate with a seed, or a tiny generator of your own (xorshift / splitmix64)?
- Key choice: start uniform, but leave room for a skewed "hot keys" distribution later.

### Acceptance criteria and test ideas
- The same seed gives identical operations; different seeds give different ones.
- Put → get; put, put → get returns the latest value; put, delete → get; get on an unknown key.
- A seeded run of about 100k operations finishes quickly without panicking.
- `cargo test`, `cargo clippy`, and `cargo fmt --check` all pass cleanly.

### Deferred
Disk, durability, networking, async, concurrency, timeouts, scans, transactions.

### Milestone 1 is split into small assignments

| Step | Scope | Rust Book prerequisite | Status |
|---|---|---|---|
| 1a | Sequence-number type in `db` | ch. 5 | **active** |
| 1b | Operation type (put/get/delete) | ch. 6 | — |
| 1c | Reference model | ch. 8 | — |
| 1d | Seeded deterministic workload generator | ch. 5–8 | — |

#### 1a: The sequence-number type

- **Problem:** every change gets a position in the log, and every later component (WAL, replay, storage, freshness reads) passes these positions around. A dedicated type lets the compiler catch mix-ups, such as passing a key count or a byte length where a sequence number belongs.
- **Reading:** Rust Book ch. 5.1 (tuple structs), 5.2 (`#[derive(Debug)]`), 5.3 (methods, associated functions).
- **Required behavior:** a type in `db` that represents a sequence number. It can be created from a `u64`, the `u64` can be read back, it can produce the next sequence number, two of them can be compared for equal / less than, and it can be printed with `{:?}`. Remove the placeholder `add` function.
- **Invariants:** next(n) > n. A plain `u64` can't be passed where a sequence number is expected.
- **Decisions to make and explain:** tuple struct vs named field; public or private inner value; what 0 means; what happens when next() is called at `u64::MAX`; whether it should be `Copy`; which traits to derive.
- **Acceptance:** tests in `db` cover next, ordering, round-tripping the `u64`, and Debug output. Manually confirm that passing a raw `u64` where the type is required fails to compile. `cargo build`, `cargo test`, and `cargo clippy` are clean.
- **Deferred:** encoding to bytes, the WAL assigning numbers, concurrency.

### Pre-work questions
1. Why does the paper's tail latency come from per-page log chain length rather than total log volume?
2. What will get return for a deleted key, and why?
3. Which crates will the workspace have by Milestone 5, and what belongs in the shared library?

---

## Correctness questions log

Record decisions here as each one is settled (from `CLAUDE.md`):

| Question | Decision | Milestone |
|---|---|---|
| What failures does "durable" cover; when may a write be acknowledged? | — | 2 |
| Client timeout after a persisted write? | — | 5 |
| Retries and duplicate replay? | — | 4 |
| Crash consistency of state + applied position? | — | 3 |
| Incomplete log records vs corruption? | — | 2 |
| Replay slower than incoming writes? | — | 6–7 |
| When may old log data be removed? | — | later |
| Read guarantees when components are down? | — | 6 |
