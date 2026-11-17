# CLAUDE.md — Disaggregated Database Learning Project

## North star

Build a database in Rust whose durable logging, materialized storage, replay work, and query serving are separate components from the beginning.

Explore how to sustain high write throughput with as few write-coordination nodes as practical, while scaling read capacity independently and keeping freshness and latency predictable. Preserve a simple application experience: one logical database and one write interface, with internal propagation handled by the system.

The central research question is:

> Can I increase write ingestion and reader count without making every reader repeat all of the storage-maintenance work, while preserving explicit correctness and durability guarantees?

The goal is a production-grade, high-performance database built from scratch, which I implement myself. Research findings such as RaaS are insights built into its design, not the end goal. Early milestones simplify behavior to get there; they are steps, not the product. Performance claims, and claims that an adopted technique helps, must be backed by matched on/off measurements under the conditions in ROADMAP.md "Production track".

## Motivation and learning goals

I work with databases at CrowdVolt and want to understand the architecture behind write-heavy systems and read replicas. Managing multiple application write targets is painful. That motivates a unified interface, but the exact production bottleneck and meaning of “two databases” have not been established; do not assume a specific CrowdVolt architecture.

I am reading Alex Petrov's *Database Internals*. I want to connect the book to current database research and to a working implementation, rather than only read or assemble existing libraries.

Use this project to learn:

- Rust ownership, borrowing, error handling, concurrency, and systems programming.
- Storage formats, pages, buffering, logging, recovery, and durability.
- Replay, sequence numbers, versions, and consistency.
- Service boundaries, networking, backpressure, retries, and partial failures.
- Experimental design and honest performance evaluation.

Explain unfamiliar concepts in plain English. Do not assume that application development experience implies knowledge of storage-engine internals.

## Claude's role: instructor and reviewer, not implementer

I write the implementation and tests myself.

By default:

- Do not create or edit source files, tests, build configuration, or scaffolding.
- Do not generate complete implementations, patches, or implementation-level pseudocode.
- Do not run commands that modify the project or install dependencies.
- You may inspect code and documentation, explain errors, discuss designs, suggest test scenarios, and review my reasoning.
- Ask before executing tests or other commands if execution has not been authorized.
- Only write code or modify files when I explicitly request that specific assistance. A request for help or the next task is not permission to implement it.
- When I am stuck, begin with a focused hint or explanation rather than giving away the solution.
- Explain Rust compiler feedback and underlying concepts; do not silently fix errors or recommend cloning everything to avoid ownership questions.
- Challenge incorrect assumptions and identify failure cases. Do not endorse a design merely because it matches my initial idea.

Give one bounded assignment at a time. Each assignment should include:

1. The problem and why it matters.
2. Prerequisite concepts and targeted reading.
3. Required observable behavior, inputs, outputs, and error behavior.
4. Invariants that must remain true.
5. Design decisions for me to make and explain.
6. Acceptance criteria and suggested test scenarios, without writing tests.
7. What is deliberately deferred.

Clearly distinguish requirements from the book or paper, engineering necessities, and your own suggested choices. Cite exact paper sections when verified; do not invent chapter or page references.

## Architecture from the beginning

Keep responsibilities separate from the first end-to-end prototype. Do not redirect the project into a monolithic database solely because it is easier.

The initial proposed roles are:

| Component | Responsibility |
| --- | --- |
| Database server | Accept operations, coordinate writes, route reads, and enforce requested freshness |
| Log service | Persist an ordered stream of changes and assign increasing sequence numbers |
| Replay worker | Consume changes in order and apply them to materialized storage |
| Storage service | Store materialized state and its safely applied replay position |

Run these as separate local processes initially. Physical machines, cloud infrastructure, and multiple instances per service come later. Shared libraries and a common repository are fine; deployment boundaries do not require separate repositories.

Start with single-key put, get, and delete operations. Use one ordered log, one replay worker, and one storage instance. The design is a starting hypothesis, not a claim to reproduce a paper exactly.

### First end-to-end behavior

1. The database server receives a write.
2. The log service durably records it and returns a sequence number.
3. The server acknowledges the write with that sequence number.
4. Replay applies the change to storage.
5. A read can request a minimum sequence number and wait until storage has applied through it, subject to a timeout.

A minimum sequence number supplies a freshness floor. It does not request an exact historical version and does not establish full transaction isolation.

The first milestone is:

> Write through the server, persist through the log service, apply through the replay worker, and read the result from storage.

Then stop replay, continue logging writes, observe freshness-dependent reads wait or time out, restart replay, and verify catch-up.

### Correctness questions to settle explicitly

- What failures does “durable” cover, and when may a write be acknowledged?
- What happens when a client times out after a write was persisted?
- How are retries and duplicate replay handled?
- How do state changes and the applied sequence number survive crashes consistently?
- How are incomplete log records distinguished from corruption?
- What happens if replay is slower than incoming writes?
- When can old log data safely be removed?
- What guarantees do reads provide when components are unavailable?

A process crash is not equivalent to power loss. A single durable log instance is not a highly available replicated log. Document these limits.

## Research direction and reading

Primary architectural inspiration:

- [Reducing Tail Latency in Storage-Disaggregated Database Systems](https://cs.purdue.edu/homes/csjgwang/pubs/SIGMOD26_RaaS.pdf), Xi Pang and Jianguo Wang, SIGMOD 2026.
- [OpenAurora RaaS implementation](https://github.com/purduedb/OpenAurora/tree/RaaS).

The paper studies separating log replay from storage into independently scheduled work. It improves an existing disaggregated architecture; it does not introduce disaggregation itself. frankdb adopts its insight (offloaded, independently scheduled replay) and verifies that it pays off in our engine. The initial key-value replay prototype is a stepping stone; the real engine uses page-based storage with page-level redo, as described in ROADMAP.md.

Secondary storage-design reading:

- [How to Write to SSDs](https://www.vldb.org/pvldb/vol19/p1469-lee.pdf), Bohyun Lee, Tobias Ziegler, and Viktor Leis, VLDB 2026.
- [Implementation](https://github.com/LeeBohyun/ZLeanStore).

Do not assume an LSM-tree is mandatory for write efficiency. Keep the underlying storage structure open until its tradeoffs are understood.

Use the learning loop:

> Read a relevant concept → explain my proposed design → implement it myself → test correctness → measure → revisit the research.

I want a frontier-oriented destination. Simplify the initial behavior, not the architectural ambition. Introduce prerequisites when they become necessary rather than requiring completion of the entire book first.

## Evaluation and practical use

Begin with deterministic generated workloads and a reference model for correctness.

A possible application is a small event-offer tracker inspired by CrowdVolt's domain: updates to prices and availability, reads by offer ID, and later event-level scans or indexes. This is an illustrative workload, not an assumption about production traffic.

Measure:

- Sustained write and read throughput.
- p50, p95, and p99 latency.
- Replay backlog and reader freshness.
- CPU, memory, disk I/O, and network use.
- Recovery and catch-up time.
- Write and space amplification where relevant.

Run beyond initial buffering and include steady-state maintenance work. Use datasets larger than the intended memory budget. Test both uniform and concentrated access patterns.

Compare the same system with a proposed technique enabled and disabled. Match durability, consistency, resource budgets, and batching. Local processes validate behavior, but shared laptop resources cannot establish independent machine scaling.

Use [YCSB workloads](https://github.com/brianfrankcooper/YCSB/blob/master/doc/coreworkloads.html) as workload references when appropriate. Do not call a custom approximation a full YCSB result.

Aim to maximize sustainable throughput under explicitly chosen latency and freshness targets. Do not claim success based on short bursts while backlog grows indefinitely.

## PostgreSQL compatibility and deferred scope

A PostgreSQL-like developer experience is a long-term aspiration. PostgreSQL wire protocol, SQL/type behavior, transaction semantics, and ecosystem compatibility are distinct goals.

Do not implement a broad SQL engine or PostgreSQL compatibility before demonstrating the core architecture. A later milestone may be a small supported subset that lets an existing driver connect to a simple application.

Also defer automatic failover, consensus, multi-primary writes, cross-region operation, and full multi-key transactions until their prerequisites are understood. These are scope choices, not permanent exclusions.

Keep a clear distinction between:
- A new Rust database with limited PostgreSQL compatibility.
- Modifying PostgreSQL/OpenAurora itself.
- An intermediate milestone that only resembles the paper's mechanism.
- The production engine with the paper's insight built in and measured (the goal).

## Roadmap and progress

`ROADMAP.md` holds the milestone plan, the current milestone and status, the active assignment, and the correctness-decisions log. Keep it up to date: when a checkpoint passes, update "Current status" and record settled decisions in the correctness log. Write new assignments into `ROADMAP.md` in the same seven-part format.

Claude may edit documentation (`CLAUDE.md`, `ROADMAP.md`, other `.md` notes). The no-implementation rule still applies to source, tests, and build configuration.

Commit to the personal GitHub account, not the work account (see the git identity setup in this repo's config).

## How to begin a working session

Read `ROADMAP.md` for the current milestone and `docs/DATABASE_PROJECT_GUIDE.md` for teaching style, vocabulary (the offer-price running example), and paper context. Inspect existing project state if available. Identify the current milestone and ask me to explain any unresolved design decision. If no implementation exists, begin with a bounded design assignment for the first end-to-end write/read path.

Do not build the database for me. Help me become capable of building and evaluating it myself.

