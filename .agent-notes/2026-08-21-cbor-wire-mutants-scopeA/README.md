# CBOR-wire mutation campaign, scope A: survivors and dispositions

The full scope-A sweep of the CBOR-wire mutation campaign ran overnight on
ox-east-1 (2026-08-20 20:48 UTC to 2026-08-21 02:57 UTC, cargo-mutants
27.1.0, jobs 16): **3141 mutants tested — 126 missed, 960 caught, 2055
unviable, 0 timeouts**. This note is the disposition record for all 126
survivors: raw artifacts under [raw/](raw/), classification below, and
per-cluster analysis of the open findings in the numbered section files.

## Provenance, and one caveat that shapes the whole note

- Command and configuration: [raw/run-scopeA.sh](raw/run-scopeA.sh) — the six
  `--file` scopes of the campaign brief under the configuration of record in
  `.cargo/mutants.toml` (nextest, `--all-features`, dev profile), minus the
  two exploded tuple constructors deferred to scope B.
- Run metadata: [raw/lock.json](raw/lock.json); final summary and tail:
  [raw/scopeA-log-tail.txt](raw/scopeA-log-tail.txt).
- Survivor list, verbatim: [raw/missed.txt](raw/missed.txt). The full
  `mutants.out` (caught/unviable lists, per-mutant logs and diffs, 12 MB
  debug log) stays on ox-east-1 at
  `/home/agent/build/rumors-mutants/scopeA/mutants.out`; it is not under any
  retention guarantee, so anything load-bearing beyond the survivor list
  should be copied out before that box is cleaned.
- **The tree under test was commit 02560b1f** ("Retire the wire and
  payload-depth design docs...") — verified by checksum-comparing the synced
  remote tree against candidate commits, not taken from any label. The
  `fix/mutants-dispositions` branch had already advanced to 62447263 before
  the run *started*: its three disposition commits (6ed8982c, 4dfd879d,
  62447263) dispose an earlier, narrower survivor batch. The overnight sweep
  therefore re-surfaces every survivor those commits already handled,
  alongside the genuinely new ones. The classification below separates them.
- **Scope B has not run.** [raw/run-scopeB.sh](raw/run-scopeB.sh) (the
  viability sample over `Work<B>::initiator_level`/`responder_level`'s ~45k
  replacement mutants) sits ready on the box with no output directory yet.

Classification method: `cargo mutants --list` at 62447263, run with and
without the roster config, matched against the survivor list by (file,
function, replacement) ignoring line drift; roster attributions checked
against the entry text in `.cargo/mutants.toml`; every claimed killing test
verified present by name at 62447263. What is *not* verified here: that the
landed tests actually kill their mutants under mutation — that needs the
confirming re-run at the branch tip (see next steps).

The cluster section files were drafted by five parallel read-only analysis
agents working against 62447263 (one per cluster, briefed with the ladder
and the model of record); the coordinator read each in full and re-verified
the load-bearing structural claims against the code — the claimed
equivalences, the refactor targets' surrounding control flow, the
killed-at-tip re-classification, and the constant arithmetic. The killing
property families are designs, not implementations: every generator and
oracle described is a proposal for the disposition work, and any number
appearing in a design (a constant's value, a boundary) must be re-derived
when it lands somewhere enforced.

## Survivors already disposed at the branch tip (49)

### Refactored out of structural existence (3)

These no longer appear in `cargo mutants --list` at 62447263; commit
6ed8982c removed the mutable surface itself.

| Survivor (at 02560b1f) | How it dissolved |
|---|---|
| `src/observe.rs:154` replace `SessionObserver::elected` with `()` | The default is an empty body over an unused parameter; a no-op body admits no replacement mutant. |
| `src/bookmark/format.rs:89` replace `<<` with `>>` | The pre-shifted `MAJOR_*` constants are plain literals (`0x00`, `0x40`); the shift arithmetic was the only mutable surface. |
| `src/tree/mirror/framing.rs:41` replace `*` with `+` | `PAYLOAD_CHUNK_LEN` is the plain literal `0x1_0000`. |

### Roster exclusions (21)

Suppressed by `.cargo/mutants.toml` at 62447263 (the tip `--list` drops
exactly these 21); each entry carries its equivalence or work-only proof in
the roster itself, so only the pairing is recorded here.

| Survivors | Roster entry |
|---|---|
| `bookmark/format.rs` `push_head` `\|`→`^` × 5 (lines 240, 242, 246, 250, 254) | push_head bit-disjointness: pre-shifted major, sub-32 right operands. |
| `tree/mirror/cbor.rs` `write_head` `\|`→`^` × 5 (lines 72, 73, 75, 79, 83) | write_head bit-disjointness (same argument). |
| `tree/mirror/handshake.rs:88` `>`→`>=` | `PREAMBLE_MAX` const-max tie-break: both arms equal at a tie. |
| `tree/mirror/handshake.rs:257` `<`→`==`, `<`→`<=` | `decode_v2`'s defensive width guard: unreachable on admitted inputs; kept a graceful error by recorded design. |
| `streaming/window.rs:442` `*`→`/` | Leaf-request edge population is zero for every representable corpus; divisor a nonzero constant. |
| `streaming/window.rs:708` `>`→`>=`; `714` `>`→`>=`/`<`/`==` | `leaves_quantile` widened root guard and fallthrough-mean legs: movement invisible past the clamp. |
| `streaming/window.rs:759` `-`→`+`, `-`→`/` | `stage_population` occupied-depth legs: only the corpus term of the closing min ever binds. |
| `src/peer.rs:722` replace `warm_caches` with `()` | Performance genre: moves only when the lazy memos compute. |

### Killed by tests landed in 6ed8982c / 4dfd879d / 62447263 (24)

Each killing test is verified present at 62447263 by name; the kills
themselves await the confirming re-run.

| Survivor (at 02560b1f) | Killing test |
|---|---|
| `message.rs:111` `PayloadDepthLimit` Display → `Ok(Default)` | `depth_limit_displays_its_steps` (`src/message/tests.rs`) |
| `message.rs:586` `Message` Debug → `Ok(Default)` | `debug_shows_the_cached_serialization` |
| `message.rs:607` `Message` hash → `()` | `hash_reads_the_content`, with `eq_implies_hash_eq` stating the true family (the general content-sensitivity form is false in principle: collisions exist) |
| `message.rs:599` `Message` eq → `true` | `distinct_payloads_are_unequal` |
| `observe.rs:281` `Attachment` Debug → `Ok(Default)` | `attachment_debug_reports_attachment` (`src/observe/tests.rs`) |
| `peer.rs:190` `Peer` Debug → `Ok(Default)` | `peer_debug_is_a_summary` (`src/peer/tests.rs`) |
| `bookmark/format.rs:249` delete wide arm; `250`, `254` `\|`→`&` | `framing_round_trips_across_head_widths` (`src/bookmark/format/tests.rs`) |
| `bookmark/format.rs:346`, `350` delete `Reader::head` arms 26/27 | same family, plus `foreign_versions_are_rejected_by_value` for the widths payload length cannot reach |
| `tree/mirror/cbor.rs:232` delete `extension_len` arm 28..=30 | `ingress_paths_agree_on_every_initial_byte` (`src/tree/mirror/cbor/tests.rs`): the deleted arm diverts reserved bytes to the Indefinite defect, disagreeing with the slice grammar |
| `handshake.rs:189` `&&`→`\|\|`, `<`→`<=` in `decode_legacy` | `legacy_dialect_detection_is_exact` and `arbitrary_legacy_preamble_decodes_by_the_oracle` |
| `handshake.rs:387` `Staged::is_empty` → `true` | `dropping_mid_preamble_idle_poisons_the_link` (`tests/gossip_when.rs`) |
| `party.rs:197` EOF match guard → `true` in `read_head` | `non_eof_read_error_passes_through_as_io` (`src/tree/mirror/party/tests.rs`) |
| `materialized.rs:886` radix match guard → `true` in `absorb`; `materialized.rs:894` delete the wrong-radix `Supply` arm | `terminal_absorb_rejects_a_mismatched_radix` (`src/tree/mirror/streaming/materialized/tests.rs`) — its assertion is variant-exact (`Violation::InvalidSupply`), so the arm deletion's reroute to `UnfinishedReply` fails it too (analysis in [04-materialized-work.md](04-materialized-work.md)) |
| `stats.rs:316` `-`→`+` in `CountedRead::poll_read` | `counted_read_counts_any_chunking`, with the split-delivery witness |
| `tasks.rs:48` replace `park_after_published_error` with `()` | `park_after_published_error_parks_only_on_failure` |
| `window.rs:442` `*`→`+` in `from_budget` | `knife_edge_budgets_solve_exactly` |
| `window.rs:658` `<`→`<=` in `small_mean_quantile` | `zero_quantile_shortcut_is_exact` |
| `window.rs:751` `\|\|`→`&&` in `stage_population` | `boundary_stages_are_empty` |
| `backend/local.rs:39` typed `Node::hash` → `Default::default()` | `erased_observations_match_the_typed_node` |

### Already under investigation (1)

`streaming/materialized/unknown.rs:92` delete `!` in `unknown` — the
leaf-verdict survivor — has its own record in
[the unknown-pruning-survivor note](../2026-08-21-unknown-pruning-survivor/README.md),
whose root cause (deep-branch geometry unreachable through blake3-derived
paths) generalizes into
[the collision-schedule test mode design](../2026-08-21-collision-schedule-test-mode/README.md).
Planned disposition: a targeted forged-path proptest family at the module,
plus the collision-schedule mode as the suite-wide instrument.

## Open survivors (77): analysis by cluster

Each cluster's full analysis — per-mutant reachability, why the suites
missed it, and the disposition with generator/oracle designs — lives in its
section file. Every disposition follows the roster header's ladder
(refactor, assert, roster exclusion, killing property family, in that
order of preference).

- **[01-capture.md](01-capture.md) — the CBOR reflection renderer (34).**
  One structural cause: the renderer is total over CBOR while every
  committed fixture speaks only the protocol's dialect (no floats, simples,
  negative ints, unnamed tags, or containers nested under map/array arms),
  and the negative-path tests assert a reason-independent fallback
  substring. All diagnostics-integrity gaps — but two survivors (the
  float-width miscount, the trailing-byte swallow) directly void the
  injectivity claim that licenses "a rendering with no hexdump is still a
  byte pin." Dispositions: 1 refactor (a vacuous short-listing clause), 1
  roster candidate (a bit-disjoint accumulator, the `write_head` proof
  verbatim), 32 killed by six families headlined by a `parse_node` <->
  ciborium differential oracle and an exhaustive major-7 byte-space spec.
- **[02-codec.md](02-codec.md) — wire parsers and sizers (23).** Three
  causes: self-denominated constant oracles (tests asserting a constant's
  consequences in terms of the constant), half-misdial blindness (every
  `major != M || value != V` guard tested only with wrong-both shapes),
  and coincidental const equivalence (`2 + 2 == 2 * 2`; `head_len` flat
  across the mutated arguments). Dispositions: 6 refactors to literal
  constants with derivations moved into tests (the 6ed8982c precedent),
  17 killed by three instruments — the **decode-canonicity oracle**
  (`decode(bytes) = Ok(v) ⇒ encode(v) == bytes`, per wire surface), the
  head-misdial matrix feeding it, and a lone-record boundary family.
  Several `&&` legs verifiably make the wire *accept* non-canonical
  spellings (conformance register); `SUPPLY_FRAME_OVERHEAD`'s `+ → *` is a
  real one-byte interop drift masked only by the encoder and decoder
  sharing the constant.
- **[03-proxy-adapter.md](03-proxy-adapter.md) — proxy, adapter, stream
  labels (8).** One proven-equivalent comparison dissolves into
  `match next.cmp(&radix)`; seven die to five families (label-byte matrix
  differential against `cbor::read_head`, an opening-supply grammar
  recognizer, a misbehaving-backend adequacy family for the `validate_leaf`
  trust-boundary assert, the `Early` pairing property, a terminal-encoder
  contract family). Headline: the surviving `!batch.is_empty()` inversion
  in `terminal` is hard evidence that **no committed session ever delivers
  a scope to the terminal stage** — independent confirmation of the
  collision-schedule design's geometry-shadow claim for bottom-level walks.
- **[04-materialized-work.md](04-materialized-work.md) — walk, answer,
  resolver, levels (8 open, after re-classifying the ninth as killed
  above).** Two `leaf_parent` guards are equivalent at their sole call
  site (empty listings routed away upstream; all-match joins unreachable
  because equal leaf sets imply equal hashes) — refactor to unconditional
  counting with the invariant stated and asserted. The resolver's ordering
  guards collapse into one `Ordering` match (dissolving the equivalent
  `>=` leg), with a fault-vocabulary extension killing the arm-level
  residue. The three `internal_walk` height mutants die to one
  strengthened per-role window-conformance family — today every
  window-scaled queue edge is tested only at capacity 1, so the fixed-
  memory envelope's per-height pricing is entirely unpinned.
- **[05-misc.md](05-misc.md) — stats, alternating backend (4).**
  `CountedWrite::poll_shutdown` is dormant (nothing in the crate ever
  shuts a mirror write half down) — a proxy-transparency family over both
  counting wrappers closes the whole trait surface. The alternating
  remote's EOF arm is a verbatim duplicate of a killed codepoint —
  refactor to a shared helper. The two partition survivors sit in the
  blake3 geometry shadow; one is bug-shaped: `close`'s Done exit computes
  the reconciled root's **causal ceiling as `|` (join)**, and the `&`
  (meet) mutant under-joins the deletion-honoring boundary — unreached
  because both committed leaf-parent pins exit via Continue. Kill with a
  generalized forged-geometry leaf-parent scenario family in `tree::arb`
  (subset/superset × flanking neighbors × distinct versions, union-tree
  oracle), the deterministic pins staying as witnesses.

Cross-cluster verdict: nothing threatens convergence between honest peers
under the model of record. The sharpest items are the conformance register
(non-canonical spellings silently accepted under several codec `&&` legs;
the resolver's supply-ordering detector bypassable), the two injectivity
breaks in the capture renderer's pin, the under-joined redaction ceiling in
the alternating close, and the untested per-height capacity pricing.

## Next steps

1. **Confirming re-run at the branch tip.** Every "killed by landed test"
   row above is a claim verified only to the level of "the test exists and
   reads the mutated behavior"; the run that demonstrates each kill is a
   scope-A re-run at 62447263 (or wherever the disposition work lands), on
   ox-east-1. The same run is the adequacy check for the whole first batch.
2. **Run scope B.** The script is staged on the box; it only classifies
   viability and skips the baseline, so it is cheap relative to scope A.
3. **Dispose the open clusters** per the section files. The shared
   instruments do most of the killing and generalize past these mutants:
   the decode-canonicity oracle plus head-misdial matrix (codec, greeting,
   labels — every future `major/value` guard is born covered), the
   ciborium differential oracle (capture), the per-role window-conformance
   family (levels), and the forged-geometry leaf-parent scenario family
   (partition). The refactors (about a dozen sites) are adoptable-now,
   behavior-preserving.
4. **Geometry-shadowed sites need the collision-schedule mode eventually.**
   The terminal-encoder pair and both partition survivors are killable now
   by scripted/forged-geometry families, but the mode is the instrument
   that removes the per-site foresight; when it lands, the campaign's test
   command must include the collision leg (the mode's design note already
   records this requirement). Notably, the materialized-work cluster needs
   it for none of its nine — the `leaf_parent` equivalence survives the
   mode by its full-width-injectivity guarantee.
5. **Audit the `Violation` vocabulary against the `Fault` harness.** Two
   clusters independently converged on the same lesson: every `Violation`
   variant wants a fault injection that provokes it and asserts the exact
   variant. Several variants appear to have none today (see
   04-materialized-work.md's cross-cutting notes).
