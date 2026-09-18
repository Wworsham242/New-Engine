# Phase 004 â€” International Physical Energy Trade

## Purpose

Phase 004 creates the first cross-country causal seam.

Energy remains the authoritative owner of physical energy imports/exports,
consistent with the architecture baseline.

No generic Trade subsystem is introduced merely to pass numbers between
countries.

## New state

```text
Country x Country x EnergyType
```

physical trade is represented with `Dense3`.

The state distinguishes:

```text
planned physical trade
realized physical trade
```

A planned flow is a commitment/request.

A realized flow is constrained by the exporting country's physical production
of the relevant energy type.

## Conservation

Every realized flow is stored once as:

```text
origin -> destination
```

The same quantity is therefore simultaneously:

- an export for origin;
- an import for destination.

The kernel checks the aggregate physical trade conservation error after commit.

## Phase-004 demonstration

The engineering fixture uses:

```text
SAU -> USA oil = 30
USA -> CAN oil = 30
```

Baseline balances are constructed so all three countries have sufficient
physical energy.

A primitive shock then applies:

```text
USA oil capacity -50%
```

The USA can no longer fulfill its planned oil export to Canada.

This creates the first endogenous international chain:

```text
USA physical capacity shock
 -> lower USA oil production
 -> lower realized USA -> CAN oil flow
 -> lower Canadian energy availability
 -> Canadian shortage/price response
 -> Canadian GDP response
 -> Canadian fiscal revenue response
```

There is still no scripted cross-country GDP penalty.

## What remains intentionally simplified

- no market-clearing trade prices;
- no alternate supplier search;
- no transport capacity;
- no sanctions;
- no shipping transit delay;
- no exchange rates;
- no monetary trade balance;
- no inventories;
- no strategic stockpiles.

Those features belong in later phases and ADR-005-style delayed mechanisms.

## Reference policy

This is original engineering scaffold logic.

IFs remains the system-of-systems architectural/behavioral reference.

CMO remains a reference for later operational military logistics and physical
interdiction, which will eventually be capable of damaging or interrupting these
trade routes without directly scripting civilian outcomes.