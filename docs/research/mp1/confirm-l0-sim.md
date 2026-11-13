VERDICT=PASS

# Confirmation of L0-SIM at c0ab357 (performance-architect)

HIGH=0. All runs are listed with their labels. The lane is clean at c0ab357.

## 1. N1: closed
- Scratch test (e) (rate 1/2, no RateQ) panics: "the rate 1/2 does not divide Q = 1 of job 1 …" (`production.rs:389-400`).
- Builder tests at c0ab357: 11 of 11 production tests green. 262 of 262 sim lib tests passed, 1 ignored (`mp1-l0sim-confirm-head`).
- The 2 x duration case is covered: my plant (below) turns `a_job_queued_at_q_2_at_rate_1_2_takes_two_times_its_duration` red.
- Bytes are the same at Q 1:
    - The trace bodies are cmp-equal to bfea57c.
    - The stream bcbf72c2… and `probe_neutral` d0d1b1c572163424 / aa9db740578ed5c1 are equal.
    - The f11 economy log (Forge: machines and research) at 665a6e7 and at c0ab357 has an equal first 300 MB (cmp).
- The panic is not a High.
    - It cannot be reached in any build before L-PWR. `power::rate_factor` returns 1/1 in production (`power.rs`; `PowerProbe` is cfg(test)). `*.when_short` is refused as unbuilt (`turts_mod_runtime/src/unbuilt.rs:62-66`). Duration 0 is refused (`content_m3_validation.rs:67-68`), so the Q fallback in `job_step` cannot be reached.
    - Condition for L-PWR: the packer computes Q as the least common multiple and refuses a Q above 65,536. A test asserts that every Rate of a packed catalog divides Q. Whether a live match gets an integrity stop in place of a process panic is an L-PWR or L-ROOM decision.
- My fault on the total path: the clamp `elapsed.min(job.total)` → `elapsed.min(job.duration_ticks)` (`production.rs:342`).
    - Red: 3 tests (low rate at every start phase, rate change, Q 2 at 1/2).
    - Restored, checked with cmp against `git show c0ab357`.
    - Green: 11 passed.

## 2. N2: closed
- `the_script_and_the_admission_system_have_one_order_when_both_are_installed`: ok (`mp1-l0sim-confirm-checks`).
- The order (script after admission, `command_script.rs:45-52`) is right.
    - None of t5, hd1 and f11 uses `--command-script` (grep). The `command_script` acceptance test has no client.
    - So no present test has live and scripted commands in one tick.
- Condition for L-MATCH: the input log must record the commands in `CommandQueue` order (live, then script), or a replay of a mixed tick (MAT1 h) can differ.

## 3. Trace headers: verified
- bfea57c, emitted in a scratch copy with a forced rebuild: the whole-file hashes equal the recorded values. Header e5152ece… = sha256 of `m2_trace.rs` at bfea57c.
- c0ab357 (lane): files base bf2a93ed…, haul 0fd2e8eb…, acme fb13c969…, cedar 0383a11b…. Header 04caf579… = `sha256sum crates/turts_pack/tests/m2_trace.rs` (`m2_trace.rs:49`, `:67`, `:500`).
- `tail -n +2` bodies are cmp-equal for all four. Line 1 is equal once the `emitter_sha256` field is removed. Line counts are 2,898 / 4,820 / 802 / 4,712.
- Body sha256 values to record:
    - base d78dc09392e5b96917513772488991b617e8d6bb2b70a26056075c28c4822f0c
    - haul 887ce95f698017026f79685347446e5240f32770e704af9d097e06eef86b074f
    - acme ba1c849aaee46dd490b9d7d444c4c431bc324b4970e6f5c6edb31a565bd19856
    - cedar 09f637325c5119727a70286e1b47b0a223cf60ec1a206aebc35853a5ffe540ae
- Can the five new arms change an M2 or M3 run? No.
    - They are new cases of an exhaustive match in `behaviour_name` (`m2_trace.rs:141-158`), reached only by an order with a 0.2.4 behaviour.
    - No file of mods/base, haul, acme or cedar names Construct, CancelConstruction, SetRally, Repair or Surrender (grep).
- Holes in the ruling, and the fixes:
    - (a) Record the four body hashes above. Today only the whole-file hashes are recorded.
    - (b) Keep "header = sha256 of the emitter file" as a check. A header change must then come with a reviewed diff of `m2_trace.rs`.
    - (c) The body rule cannot see an emitter change that only matters for a package outside the four. That is the same as before the ruling.

## 4. Merge ratchet: exact
- `diff` of the BASELINE at bfea57c against 0e860dc shows exactly the six relation_path changes:
    - core_adapter.rs and cursor.rs rows removed;
    - combat.rs [0,3,4,0,0] → [0,0,4,0,0];
    - run.rs row removed;
    - relation.rs [0,5,0,0,0] added;
    - rules_adapter.rs [0,5,0,0,0] added.
- Nothing raised. The ratchet at c0ab357 has 7 passed.

## 5. Acceptance steps alone
- t5 at c0ab357: 2 of 2 green, lag_p99 1.349 and 1.491 (limit 1.5) at load average 29-36 on 28 CPUs. Under load the margin is 0.009.
- hd1 native reference at c0ab357: 2 of 2 green.
- f11 at c0ab357:
    - Run 1 (load 26-36): FAIL, "timed out waiting for live client report …/owner-live.json" after 230 s. The file appeared later.
    - Run 2 (load 18-21, `mp1-l0sim-confirm-f11-c0a`): green in about 90 s.
- f11 at 665a6e7 (`mp1-l0sim-confirm-f11-665`, load 15-17): green.
- The economy log prefixes are equal, and the three steps use no script. Neither the merge nor N1/N2 can cause a fault here: only host load.
- The f11 economy log grows about 9 MB/s (829 MB in a green run). That I/O, while other lanes compile, is a likely source of the timeout (NOT TRACED).

## Instrument note
Two `git archive` copies that share one CARGO_TARGET_DIR can reuse stale artefacts: the archive mtimes are old, and the path metadata is equal. I saw 133 false compile errors. My earlier results of this review are not affected. Every scratch result here was rerun with a fresh target, or with the sources touched.
