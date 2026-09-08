# SPEC – Named invariants and demo predicates

**Version:** 0.6
**Date:** 2026-09-08
**Status:** Fast-clock input. Oracle implements what is named here.

## Active slice

Sprint 3.1 — `docs/sprints/CURRENT.md`.

Mobile soil water m = (θ − θ_fc)+ · L may move to a lower-z neighbor or to an equal-z neighbor with smaller m. Capillary water stays. No uphill soil drain. Pond routing remains S02.1.

Tick: rate-limited infiltrate, S02.1 pond runoff, then gravity drain (S03 + S03.1).

I1–I6 unchanged. Closed-grid mass still conserved (I3).
