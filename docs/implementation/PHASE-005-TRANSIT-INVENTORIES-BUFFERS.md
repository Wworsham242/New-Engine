# Phase 005 â€” Transit, Inventories, and Buffers

## Purpose

Phase 005 implements a central New Engine requirement:

> a disruption may occur now while the largest consequence appears later.

This follows the delayed-system behavior observed in IFs and the logistics
discipline that makes Command: Modern Operations useful as a military reference.

Neither reference is copied. The implementation remains original.

## Explicit temporal mechanisms

Energy now owns three distinct physical concepts:

```text
shipment launched this tick
shipment in transit / arriving this tick
inventory held locally
```

A shipment takes one monthly master tick to arrive in the Phase-005 profile.

The delay is explicit state, not an implicit "use last tick" convention.

## Buffer behavior

Countries hold physical energy inventories.

When current production plus arriving imports minus newly launched exports
cannot satisfy demand, inventory is released before an observable shortage is
declared.

This creates:

```text
shock
 -> pipeline hides some effect
 -> inventory absorbs some effect
 -> inventory depletes
 -> shortage appears later
 -> macro/fiscal effects appear later
```

## Critical solver invariant

Numerical iteration does **not** advance physical time.

The kernel creates one immutable Energy temporal base per master tick.

Every Energy candidate in the Economyâ†”Energy solver reads the same:

- opening inventory;
- opening in-transit shipments;
- capacity;
- planned trade.

Therefore 30 numerical iterations cannot accidentally consume 30 months of
inventory or advance a shipment 30 times.

Only the committed master tick advances those states.

## Demonstration

The Phase-005 fixture applies:

```text
Month 1:
USA oil capacity -50%
```

USA exports less oil to Canada immediately, but:

1. previously launched oil is already in transit;
2. Canada receives that old shipment first;
3. subsequent lower arrivals are buffered by Canadian inventory;
4. Canadian macro effects appear only after the buffer is depleted.

This is exactly the class of delayed second-order behavior the architecture
requires.

## IFs lodestar

IFs informs the expectation that system effects can cross modules and appear
with lags, buffers, and feedback rather than as immediate scripted penalties.

## CMO lodestar

CMO informs the physical/logistical intuition:

- stocks exist;
- flows exist;
- transit exists;
- throughput constraints exist;
- current operations consume physical resources;
- a disrupted route can matter after existing supplies are exhausted.

New Engine stays one abstraction level above CMO for military entities, but
preserves those causal relationships.

## Next likely step

Phase 006 should generalize delayed state into reusable kernel infrastructure:

- stable delayed-event queue;
- typed pipeline entries;
- due-tick activation;
- canonical event ordering;
- save/hash coverage;
- tests proving solver iteration cannot advance pipelines.

That lets construction, repair, procurement, training, agriculture, and later
military logistics use the same deterministic temporal machinery instead of
inventing subsystem-specific delay hacks.