# Story 11.2 AC1 crash acceptance

Story acceptance criterion `11.2.AC1` passes for the current operational-store scope on Linux.
The retained subprocess campaign executes 224 abrupt stops over all 16 declared durable boundaries,
both before and after commit, with seven deterministic seeds in every boundary-position cell.

On reopen, the campaign admits only zero or one target row before recovery and exactly one afterward.
The new source and workflow families additionally require manifest/provenance/lifecycle atomicity,
extraction/index separation, exact cache authority, single attempt identity, one attempt across receipt,
verification, and recovery, no stale source represented as current, duplicate-publication refusal, and
no recovery effect-driver launch. Existing checkpoint, migration, key, backup, restore, expiry, and
deletion branches retain their exact pre/post-state assertions.

The acceptance result uses synthetic data and establishes the criterion for the current encrypted
operational-store transaction set. It does not represent physical power-loss, torn-sector,
storage-controller, cross-platform, installed-product, independent-review, Story, Sprint, packaging,
or release evidence.
