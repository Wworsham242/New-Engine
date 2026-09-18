# Phase 003 â€” Country-Indexed World State

## Purpose

Phase 003 replaces the single global proof-of-concept world with the first
registry-dimensioned world state.

The architecture now begins to resemble the IFs-style system-of-systems target:

```text
WorldRegistry
 -> CountryId
 -> dense country state
 -> Country x EnergyType state
 -> country-specific contracts
 -> country-specific shocks
 -> deterministic country-order coupled solve
 -> canonical country-order state hash
```

## Storage

- Economy: `Dense1<Country>`
- Governance: `Dense1<Country>`
- Demographics: `Dense1<Country>`
- Energy demand/price/shortage: `Dense1<Country>`
- Energy capacity/production: `Dense2<Country, EnergyType>`

This follows ADR-009 rather than creating one ECS entity for every
country-variable pair.

## Phase-003 demo world

The current three-country world is an engineering fixture:

- USA
- CAN
- SAU

Energy types:

- oil
- gas
- electricity

The numeric fixture values are not asserted to be real-world calibrated data.

## Shock test

The vertical slice applies:

```text
USA oil capacity -50%
```

The shock changes a primitive physical capacity.

It then propagates through:

```text
Energy
 -> energy shortage/price
 -> Economy
 -> Governance revenue
```

Canada and Saudi Arabia remain unchanged in this phase because international
trade and cross-border energy substitution have not yet been modeled.

That isolation is intentional and tested.

## Determinism

The coupled solve iterates countries in canonical ascending `CountryId`.

The state hash includes:

- tick;
- country registry identity;
- energy-type registry identity;
- dense state values in canonical row-major order.

## Reference policy

The equations remain temporary engineering scaffolds.

International Futures continues to inform subsystem architecture, causal
interfaces, and future validation behavior, but no proprietary IFs equations are
copied.

Command: Modern Operations remains a reference for later military operational
relationships at the higher force abstraction defined in ADR-008.

## Next phase

Phase 004 should introduce the first cross-country seam:

- trade/energy import-export availability;
- a minimal country-to-country flow matrix;
- explicit ownership and conservation;
- shock propagation from one country's energy loss into another country's
  import availability.

This should be done before increasing equation sophistication.