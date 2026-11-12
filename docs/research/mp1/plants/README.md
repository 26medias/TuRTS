# MP1 plant records

A plant is one planted fault. It proves that a test can go red. Each MP1 task that has plants
writes one file here: `docs/research/mp1/plants/<task id in lower case>.json`. The file is a
JSON list of records. The format comes from plan section 1 "Plant records" (ruling R-B, qa N6).
The runner is `scripts/qa/mp1_plants.py`. The parallel runner reads the same records through
`tests/mp1/refutations.py` (set `mp1`).

## The record

| Field | Type | Meaning |
| --- | --- | --- |
| `row` | string | The acceptance row or the task id that the plant serves (for example `CON1`). |
| `plant` | string | The plant id inside the row (for example `a`). `row` and `plant` together are unique in all files. |
| `files` | list | One entry for each edited file. A plant that edits two files has one record with two entries. |
| `files[].file` | string | Repository path, relative, no `..`. The file must be tracked and clean. |
| `files[].edit` | object or list | `{ "old": "<text>", "new": "<text>" }`, or a list of these. The `old` text occurs exactly once in the file. |
| `files[].patch` | string | A unified diff of this one file, instead of `edit`. Each hunk's old text (context and removed lines) occurs exactly once, at whole lines. Hunk line numbers are not used. |
| `command` | list of strings | The command, run from the repository root with no shell. Use `["bash", "-c", "..."]` when you need a shell. |
| `env` | object | Environment variables of the command (strings). `{}` when none. They win over the runner variables. |
| `headed` | boolean | `true` when the red runs through the headed wrapper. `--native-only` skips these records. |
| `expected_red` | string | The text that the red prints. One line, or several lines in one string. See "What counts as a red" below. |
| `exit` | integer | The exit code of the red. It is not 0. |
| `commit` | string | Lower-case hex commit id (7 to 40 digits): the commit of the tree on which the red was run. A task that adds the record in the same commit writes the base commit of its lane. |

| `expected_selected` | integer or string, optional | For a `cargo test` plant: the guard that a red of another test does not count (the guard of `tests/m3/refutations.py:143`). An integer: the `running N tests` lines of the output add up to N, in the green runs and in the red run. A string: the full test name; the green runs print `test <name> ... ok`, the red run prints `test <name> ... FAILED`, and no other test fails. **Set it for every cargo test plant.** |

Each entry of `files[]` has exactly one of `edit` and `patch`. A record has these fields and no other.

## What counts as a red

The red run must give the recorded exit and print the recorded text on a line that:

- is not an echo line (a line that starts with `test `, `thread '` or `failures:`). The same filter applies to each line of a multi-line text;
- is not a line of the plant's own edit (a command that prints the edited file proves nothing);
- is not printed by the green run. The runner refuses a plant whose `expected_red` also occurs in the output of the green baseline (a `set -x` echo, a note that the command always prints).

A compile error is not a red.

## What the runner checks

For each selected record (a failure at any step names the record `row/plant`):

1. Every record of every file has all the fields with the right types. Rows and plants are unique.
   This check covers all the records, in every mode, also the ones that `--files` does not select.
2. Every file of every record exists. A moved or deleted file gives `REFUSED <row>/<plant> [<json file>[<index>]]: ... re-pin the record` and exit 2.
3. A file must be tracked and clean (`git status` shows no change in it). This check comes before the anchor check.
4. Each anchor occurs exactly once. A patch that does not apply, or an anchor that occurs 0 or 2 times, gives `REFUSED` and exit 2. No command runs if any selected record is refused.
5. The command is green (exit 0) on the clean source. The plants that share a command and an `env` form a command group: one baseline for the group.
6. For each plant: the edit, then the red (see above), then the restore (`git checkout HEAD -- <file>`). Every byte is equal to the byte before the run, and `git status --porcelain -uall` shows the same lines as before the group. A new file that the command wrote (untracked) is a `PLANT-FAIL` that names the file; the runner does not delete it.
7. One final green run for the group after the last restore (two green runs for a group, not two for each plant). If it is red, every plant of the group fails.

A plant that stays green is a failure of the plant: `PLANT-FAIL <row>/<plant>: plant stayed green (exit 0)`.

## Interruption, SIGKILL and the lock

- The runner holds SIGINT, SIGTERM and SIGHUP during the restore and its check. A signal that arrives then is handled after the restore. The runner says "restored" only after the byte check passed.
- Before the edit, the runner writes a journal (`<git dir>/mp1-plants.journal`: record, file, sha256 of the original). It removes the journal after the checked restore. SIGKILL cannot be caught: the next run reads the journal first, restores each file from HEAD, checks the hash, and prints `a plant of <row>/<plant> was left in <file> by an interrupted run; restored`. Then it goes on.
- One runner in one tree: `flock` on `<git dir>/mp1-plants.lock`. A second runner exits 2 with `another mp1_plants.py run holds ...`.

## Headed records

The serial runner never runs a record with `headed: true`. In every mode it skips the record and prints `SKIPPED <row>/<plant>`. `--native-only` is kept for old command lines and changes nothing. A headed plant runs only through the headed wrapper, one call for each plant:
`/home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v6.sh --mode pre-merge|table <exec_in_lane_v4 flags> -- <command>`. An exit of 75 from the wrapper means "yield, retry later": it is not a failure of the plant.

## Commands

    python3 -I scripts/qa/mp1_plants.py --self-test
    python3 -I scripts/qa/mp1_plants.py --files <changed paths> --native-only; echo exit=$?
    python3 -I scripts/qa/mp1_plants.py --all --native-only; echo exit=$?
    python3 -I scripts/qa/mp1_plants.py --list --all          # select, run nothing

Exit codes: 0 all selected plants are red and restored (or none is selected); 1 a plant failed;
2 a record is refused or another runner holds the lock; 130 or 143 interrupted. `--files` selects the
records that have the path in `files[].file`. A path under the root may be absolute. A path that is
neither a tracked file nor in a record is refused (exit 2): it is a mistake, not "nothing to run". `--timeout S` bounds each command run (default 1200 s). `--out P` writes the results
as JSON. `--root` and `--plants-dir` select another worktree or folder (used by the self-test).

## Forward merges

A forward merge can stop a record from applying (plan section 1, qa N5). The owner of the merge
re-pins the `edit` or `patch` text of the record and runs the plants of the merged paths red in the
same merge. Update `commit` when you re-pin. A moved file shows as a `REFUSED ... re-pin the record` line with the
JSON file and the record index.

The parallel runner (`parallel_refutations_v2.py`, set `mp1`) gives each command group to one worker.
The worker has one scratch copy of the worktree and one copy of the cargo target for the whole group.
It does not run `headed: true` records: they run through the wrapper in F-HEAD and in REG2. The slow
gate uses `--set mp1`.
