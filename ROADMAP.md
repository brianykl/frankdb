# frankdb Roadmap

A storage-disaggregated key-value database in Rust, inspired by
[Replay-as-a-Service (RaaS)](https://cs.purdue.edu/homes/csjgwang/pubs/SIGMOD26_RaaS.pdf)
(Pang & Wang, SIGMOD 2026).

**What this is:** a production-grade, high-performance database built from scratch in Rust. The RaaS findings are a design insight we build *into* it (offloaded, independently scheduled replay to keep tail latency low), not the end goal.
**What this is not:** an experiment for its own sake, or a modification of PostgreSQL/OpenAurora. Milestones 1–7 are scaffolding toward the real engine.

**How the paper is used:** we adopt its insight and verify that it pays off *in frankdb*: turning the technique on vs off under matched conditions should cut tail latency for the reasons §3 gives. That measurement is how we know the design choice was right, not the product itself.

## Current status

- **Current milestone:** 1, Vocabulary types + server interface
- **Done:**
  - workspace: `protocol` (renamed from `db`), `wal`, `replay`, `server`, `storage`, `orm`
  - `protocol`: `LogSequenceNumber`, `Value`, `ColumnType`, `Record`, `Column`, `Schema` (validated: required primary key, not nullable, not float, no duplicate columns), `SchemaError`, `Request` (CreateTable, Insert, Get with optional min seq), `Response` (`Written(seq)`, `Row { record: Option, seq }`, non_exhaustive), `DbError` (non_exhaustive; `db_error_status()` -> `DbErrorStatus` Rejected/Unavailable/OutcomeUnknown; wraps `SchemaError` as `InvalidSchema`), `Executor` trait (`Send + Sync`, `execute(&self, Request) -> impl Future<Output = Result<Response, DbError>> + Send`); 13 tests passing (DbError bucket tests still to write)
  - `server`: signatures only (`main`, `listener::run`, `connection::handle`, `Engine`), bodies are `todo!()`; deps tokio, tracing, tracing-subscriber
  - Rust Book ch. 4–5 read; ch. 4 quizzed
- **Next actions, in order:**
  1. `protocol/src/wire.rs` — **where we stopped (2026-11-16):** format for `Value` is decided and written in `docs/wire-format.md`. Skeleton exists (`WireError`, tag constants, `encode_value`, `decode_value(&mut &[u8])`, private `take<'a>`). Next sitting: (a) `encode_value` + byte-exact tests, (b) read Rust Book 10.3 (lifetimes), (c) `take`, then `decode_value` + round-trip and error tests. `mod wire;` + exports already in lib.rs.
     Original plan: Request/Response <-> bytes, length-prefixed frames, **hand-rolled binary payload** (decided 2026-11-16: no serde, for learning; write the format spec down so a Python client can reimplement it). Order: `Value` encode/decode first, then Record, Schema, Request, Response, then framing
  2. fill in `server`: `Engine` implements `Executor`; `connection::handle` does read frame -> decode Request -> execute -> encode Response -> write
- **Request path (who touches bytes):** client encodes a Request with `wire` and sends it. The server's **listener** only accepts connections and hands each one to a **connection handler**. The handler decodes bytes into a `Request` (via `wire`), calls `engine.execute`, encodes the `Response` (via `wire`), and writes it back. `wire` lives in `protocol` because client and server must encode identically.
- **Reading before steps 1–2:** Rust Book ch. 6.2 (`match`), ch. 10 (traits), ch. 16 (`Arc`), ch. 17 (async); the tokio tutorial (mini-redis)
- **Known gaps to revisit:** why the borrow rules prevent dangling pointers (Vec reallocation); moves don't deep-copy; `&str`/`&[u8]` for read-only parameters
- **Open lint:** clippy suggests `Default` for `Record`
- **Open design item:** `Request::Get` still takes `key: Value`; consider a `Key` enum (Integer, Text; derives Eq/Ord/Hash) since keys can't be Float/Null. Also decide how Insert extracts a `Key` from a `Record`

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

| 11 | **Frontier:** coherent cache inside the driver | `Arc`, concurrent maps, background tasks | client-side caching, invalidation streams, fill/invalidate races | Driver cache on vs off: hit rate, p99, and zero stale reads beyond the stated freshness contract under a randomized checker |

**Later:** a page-based storage engine (B-tree vs LSM, chosen after studying both), scans and secondary indexes, YCSB-style workloads, and a small PostgreSQL wire-protocol subset.

---

## Production track (what a production-grade frankdb needs)

The milestones above build the architecture. A production-grade engine that actually benefits from the RaaS insight also needs these. They are listed so they shape earlier decisions, not as work for now.

| Requirement | Why it is necessary | Source |
|---|---|---|
| **Page-based storage + page-level redo records** | The paper's tail latency comes from per-*page* log chains replayed by GetPage@LSN (§2.3, §2.4, §3). A key-value map with whole-value puts has no chains to replay. Our patch records (Stage A+) are a per-key analogue; pages are the faithful version. | paper |
| **Storage CPU contention** | §3 attributes tail latency partly to background replay competing with foreground reads for the storage node's CPU. That only shows up if storage is a separate, CPU-bounded machine (or a strictly pinned CPU budget). | paper |
| **Separate machines** | Local processes share one laptop's CPU, disk, and memory, so they cannot show independent scaling or offload benefits. Plan for cloud VMs once the engine is ready for real load. | engineering |
| **Realistic benchmarks** | To show frankdb is fast, not just fast on a toy: YCSB-style workloads, and a SysBench-like one to compare against the paper's setting (§7). Label approximations as approximations. | engineering |
| **Matched baselines** | RaaS on vs off with the same hardware, durability, batching, and data size larger than memory. | CLAUDE.md |
| **Replicated durable log** | "Production-grade" durability means surviving a machine loss, not just a process crash. One log instance is not highly available. | engineering |

Suggested ordering choice (mine, not the paper's): finish Milestones 1–7 on the key-value engine, then switch storage to pages before Milestone 8, so the replay offloading in 8–9 is built on the structure a real engine uses.

---

## Driver-integrated cache (Milestone 11 design notes)

**The idea (Brian, 2026-11-17):** put the cache *inside the client driver* and make its invalidation part of the database's own correctness contract, instead of a separate Redis that the application keeps in sync by hand.

**Why it fits this project.** The central research question is scaling readers without every reader repeating storage work. A driver cache is the extreme case: the reader is the application process itself. And frankdb already has the two pieces a coherent cache needs:
1. **An ordered change stream** (the log). Invalidations can be derived from it, so they arrive in the same order as writes.
2. **Freshness floors** (`min_seq`). This gives the cache a precise rule for when a cached value is good enough.

**The core mechanism to design (sketch, not a spec):** the driver tracks *how far through the change stream it has seen invalidations*, call it its horizon H. Any cached key with no invalidation through H is known to be current as of H. So a read with `min_seq ≤ H` can be answered from the cache with the same guarantee storage would give. A read with `min_seq > H` must wait for the stream or go to the server. Writes return a seq, so read-your-own-writes falls out: after writing at seq s, the driver must not serve that key from cache until H ≥ s (or it caches the value it just wrote).

**Correctness questions to settle (add to the log when decided):**
- **Fill vs invalidate race.** The driver reads key k from storage at seq 10. Before the value is inserted into the cache, an invalidation for k at seq 11 arrives. If the fill lands afterward, the cache holds a stale value forever. How do fills carry their seq so stale fills are rejected?
- **Stream disconnects.** If the driver misses part of the stream, H is no longer trustworthy. Drop the whole cache, or resume from a seq the log still holds? (Ties to "when may old log data be removed?")
- **Invalidate vs update.** Push only "k changed at seq s", or push the new value? Values cost bandwidth; invalidations cost a re-read.
- **Fan-out cost.** N drivers × write rate invalidation messages. Every driver gets everything, or drivers subscribe to keys/prefixes they hold, with the server tracking who holds what?
- **Who produces invalidations?** Drivers tail the log directly, or the server/replay worker emits them? This is a new service boundary.
- **Patches.** With server-side patches (Stage A+), can the driver apply a patch to its cached record, making it a tiny replay worker?
- **Memory budget and eviction** in the application process.

**Prior art to read before designing (verified to exist; sections not yet checked):**
- *Scaling Memcache at Facebook* (Nishtala et al., NSDI 2013): leases for the stale-fill race, and invalidations driven by tailing the database commit log.
- Redis client-side caching docs (`CLIENT TRACKING`): server-tracked mode vs broadcast-by-prefix mode, i.e. the fan-out tradeoff above.
- DynamoDB Accelerator (DAX): a write-through cache in front of the database, for contrast.

**Prerequisites:** Milestones 5–7 (a real change stream, freshness reads, and a benchmark harness to measure hit rate and p99). **Checkpoint:** a randomized checker issues concurrent writes and cached reads and proves no read ever returns a value older than its freshness floor; then measure cache on vs off.

---

## Application layer: the frankdb ORM

Modeled on the py-mkt `BaseDataLayer` pattern (create / get / list / count / update / delete). Everything is in Rust.

### Decisions so far

- **One domain type = one table.** Tables are key prefixes in frankdb: an `Offer` with id `abc` is stored at key `offer/abc`, with the serialized record as the value.
- **One data layer per domain type,** built on a shared base data layer. The ORM talks to frankdb through a Rust trait in `protocol` (the shared messages and types; formerly `db`). Its first implementation is the in-memory reference model; the second is the network client.
- **Partial updates happen on the server, like Postgres.** `update(id, changes, user)` sends only the changed fields. The server writes a **patch** log record ("set price = 50 on offer/abc"), and replay applies patches in log order. No lost updates and no client-side read-modify-write. This means:
  - Records are not opaque bytes. They have named fields (start schemaless: a map of field name → typed value; add schemas later).
  - The log holds full records (puts), patches, and deletes. Reading a key may require applying a chain of patches, which is the same per-page log chain the RaaS paper studies (§2.4, §3).

### ORM ↔ server interface, in stages

| Stage | Server operations | Enables ORM methods | Lands after |
|---|---|---|---|
| A | `get(key, min_seq?)`, `put(key, value)`, `delete(key)`; writes return the sequence number | `get`, `delete`, unconditional writes | Milestone 1c (in memory), Milestone 6 (real system) |
| A+ | `patch(key, field changes)`: server-side partial update, replay applies patches in order | `update` | after Milestone 6, or earlier in the reference model |
| B | Conditional writes: only if absent / only if exists / only if version = X; returns `Conflict` | safe `create` (no duplicates), "not found" errors on `update` | new milestone after 6 |
| C | `scan(start, end, limit, min_seq?)`, `count(start, end)` over ordered keys | `list`, `count` | needs ordered storage (affects the Milestone 3 choice) |
| Later | filters, secondary indexes (`ORDER BY created_at`), multi-key transactions (py-mkt's `uow`) | full `list` filters, units of work | deferred |

**Open problem for Stage B:** conditions must be checked against the latest state, but storage lags the log. Options: wait for replay, have the log service check the condition, or have the server track recent writes. This connects directly to the paper's freshness problem.

**Possible extra:** a Postgres implementation of the same base data layer, so one application can be benchmarked on frankdb vs Postgres.

### Interface spec v0 (draft, to be finalized by Brian)

**One entry point:** `execute(request) → response or error`. The contract is the request/response *data*, not Rust-specific features, so a Python client can speak it later once there's a wire format (Milestone 5; JSON, protobuf, or custom binary to be decided then). Requests are structured commands, not a text query language. SQL is deferred. Closest real-world analog: DynamoDB's API (PutItem / GetItem / UpdateItem / DeleteItem / Query, with conditions).

**Executor trait decisions (built to last):**
- **Async from day one.** The real server waits on network and disk. `execute` returns a `Future` that is `Send`, so it works on a multi-threaded runtime. The `protocol` crate stays runtime-agnostic: it uses `std` futures only, no tokio dependency.
- **`&self`, not `&mut self`.** A real server handles many clients at once, so implementations synchronize internally. The trait requires `Send + Sync`.
- **`#[non_exhaustive]` on `Request`, `Response`, `DbError`.** New variants can be added later without breaking other crates' `match`es.
- **Errors are grouped by what the caller should do:** the request is wrong (don't retry: schema, not found, already exists); the system is temporarily unable (safe to retry: unavailable, freshness timeout); the **outcome is unknown** (a write may or may not have been logged, e.g. a timeout after sending). Unknown-outcome errors force the client to handle "maybe it happened", which is the client-timeout question from CLAUDE.md.
- **Later, additive:** batch execution, request ids for safe retries, a protocol version handshake (Milestone 5).

**Data model**

| Concept | Meaning |
|---|---|
| Table | a name, e.g. `offer`, plus a **schema** |
| Schema | an ordered list of columns; each has a name, a column type, and whether it may be null. Stored by the server in a catalog. |
| Column type | integer, float, text, boolean (more later) |
| Primary key | **declared by each table's schema and required**: exactly one column, not nullable, Integer or Text. Requests address a row by its key value. (Composite keys later.) |
| Value | one of: integer, float, text, boolean, null (more types later) |
| Record (API) | a map of column name → Value; what the ORM sends and receives |
| Row (internal) | values in schema column order, no names; what the log and storage hold |
| Seq | the `LogSequenceNumber` type from 1a |

**Requests**

| Request | Inputs | Success response | Errors |
|---|---|---|---|
| CreateTable | table, schema | seq of the write | TableExists, invalid schema |
| Insert | table, record (the key comes from the record's primary-key column) | seq of the write | AlreadyExists, schema errors |
| Get | table, key value, optional min_seq | record or nothing, and the seq it was read at | FreshnessTimeout |
| Patch | table, key value, field changes (set field = value, remove field) | seq of the write | NotFound |
| Delete | table, key value | seq of the write | NotFound? (decide) |
| Scan | table, limit, optional start-after key, optional min_seq | records in key order, plus a cursor for the next page | FreshnessTimeout |
| Count | table, optional min_seq | number of records | FreshnessTimeout |

Schema errors (Insert, Patch): unknown table, unknown column, wrong type, null in a non-nullable column, missing required column.

General errors on any request: invalid request, unavailable, internal.

**Schema decisions:** schemas are enforced by the server (validated on Insert/Patch, converted from named Record to positional Row). Catalog changes go through the log like any other write, so they survive crashes. Deferred: ALTER TABLE / schema evolution. When it comes, columns will need stable ids so dropped columns don't shift positions.

**Decisions to make**

1. ~~Get on a missing id~~ **Decided:** success with nothing (like py-mkt's `None`).
2. Delete on a missing id: error, or success that reports nothing was deleted? (py-mkt returns the deleted entity or `None`.)
3. **Decided:** the ORM sets `created_at`, `updated_at`, `updated_by`. Still open: who generates ids?
4. Paging: offset (like py-mkt) or start-after cursor? Offset re-reads skipped rows and shifts when rows are inserted.
5. Ordering: py-mkt lists newest first by `created_at`. Without secondary indexes, the server can only order by id. If ids are time-ordered (ULID / UUIDv7), id order is creation order.
6. Batch create (py-mkt accepts a list): a separate request, or defer?

**Deferred:** filters on Scan/Count, secondary indexes, text search, transactions (`uow`), row locks (`lock_row`).

**Note on Insert/Patch errors in the real system:** in the in-memory reference model, AlreadyExists and NotFound are trivial to check. In the disaggregated system, the server acknowledges after logging and storage may lag, so these checks hit the Stage B open problem.

### Server process outline

```
protocol/src/          shared by server AND client
├── request.rs, response.rs, error.rs, executor.rs
└── wire.rs            Request/Response <-> bytes, and framing (length-prefixed messages)

server/src/
├── main.rs            read config (port), build the engine, start the listener
├── lib.rs             declares the modules below
├── listener.rs        step 1: bind a TCP port, accept connections, spawn one task per connection
├── connection.rs      steps 2 and 4: read a frame, decode a Request, call execute, encode the Response, write it back
└── engine.rs          step 3: the struct that implements Executor (catalog, then WAL + storage)
```

- `listener.rs` and `connection.rs` know sockets and bytes, not tables. `engine.rs` knows tables, not sockets. The `Executor` trait is the seam.
- The wire format lives in `protocol` because the client needs the same encoding.
- **Wire format v0 (to decide):** length-prefixed frames (a 4-byte length, then the payload). Payload: JSON via `serde` (easy to debug, Python-friendly) or a binary format (smaller, faster). Start with JSON, measure later.
- **First checkpoint, the "walking skeleton":** the server binary listens on a port. A small client connects and sends a Get. The engine answers with a real `DbError` (e.g. table not found). The response comes back and is printed. Every layer exists, even though the engine does almost nothing.
- **Reading:** Rust Book ch. 17 (async) and ch. 21 (multithreaded web server); the tokio tutorial (tokio.rs), whose mini-redis example has the same listener → connection → frame structure.

### ORM side (Rust, in its own crate)

| Piece | Responsibility |
|---|---|
| Entity trait | implemented by each domain type: its table name, its id, conversion to/from a Record |
| Base data layer | generic over an entity type and a client; provides create, get, list, count, update, delete by building requests and calling `execute` |
| Per-domain data layer | e.g. offer data layer: uses the base, adds domain-specific methods |
| Client | anything implementing `execute`: first the in-memory reference model, later the network client |

Updates are data (a list of field changes), not Rust closures, because they must cross the network to the server.

### Revised Milestone 1 order

1a sequence number (done) → 1b Value, Record, Request, Response, error types (Book ch. 6) → 1c in-memory `execute` = the reference model (ch. 8) → 1d ORM v0 against the in-memory model → 1e seeded workload generator.

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
| 1a | Sequence-number type in `protocol` | ch. 5 | done |
| 1b | Value, Record, Schema, Request, Response, error types, Executor trait (see "Interface spec v0") | ch. 6, 10 | **in progress** |
| 1c | In-memory `execute`: the reference model | ch. 8 | deferred (building the real server path first; still needed later as the test oracle) |
| 1d | ORM v0 against the in-memory model | ch. 10 | — |
| 1e | Seeded deterministic workload generator | ch. 5–8 | — |

#### 1a: The sequence-number type

- **Problem:** every change gets a position in the log, and every later component (WAL, replay, storage, freshness reads) passes these positions around. A dedicated type lets the compiler catch mix-ups, such as passing a key count or a byte length where a sequence number belongs.
- **Reading:** Rust Book ch. 5.1 (tuple structs), 5.2 (`#[derive(Debug)]`), 5.3 (methods, associated functions).
- **Required behavior:** a type in `protocol` that represents a sequence number. It can be created from a `u64`, the `u64` can be read back, it can produce the next sequence number, two of them can be compared for equal / less than, and it can be printed with `{:?}`. Remove the placeholder `add` function.
- **Invariants:** next(n) > n. A plain `u64` can't be passed where a sequence number is expected.
- **Decisions to make and explain:** tuple struct vs named field; public or private inner value; what 0 means; what happens when next() is called at `u64::MAX`; whether it should be `Copy`; which traits to derive.
- **Acceptance:** tests in `protocol` cover next, ordering, round-tripping the `u64`, and Debug output. Manually confirm that passing a raw `u64` where the type is required fails to compile. `cargo build`, `cargo test`, and `cargo clippy` are clean.
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
