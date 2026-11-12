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
| `expected_red` | string | The text that the red prints. One line, or several lines in one string. |
| `exit` | integer | The exit code of the red. It is not 0. |
| `commit` | string | Lower-case hex commit id (7 to 40 digits): the commit of the tree on which the red was run. A task that adds the record in the same commit writes the base commit of its lane. |

Each entry of `files[]` has exactly one of `edit` and `patch`. A record has these fields and no other.

## What the runner checks

For each selected record, in this order (a failure at any step names the record `row/plant`):

1. Every record of every file has all the fields with the right types. Rows and plants are unique.
2. Each anchor occurs exactly once. A patch that does not apply, or an anchor that occurs 0 or 2 times,
   gives `REFUSED` and exit 2. No command runs if any selected record is refused.
3. The command is green (exit 0) on the clean source.
4. The command is red with the recorded exit and the recorded text. The text is on a line that is not
   an echo line (a line that starts with `test `, `thread '` or `failures:`). A compile error is not a red.
5. The source is restored (`git checkout HEAD -- <file>`). Every byte is equal to the byte before the
   run, and `git status --porcelain -- <file>` prints nothing. The runner restores also when the command
   fails, times out, or gets SIGINT, SIGTERM or SIGHUP.
6. The command is green again.

A plant that stays green is a failure of the plant: `PLANT-FAIL <row>/<plant>: plant stayed green (exit 0)`.

## Commands

    python3 -I scripts/qa/mp1_plants.py --self-test
    python3 -I scripts/qa/mp1_plants.py --files <changed paths> --native-only; echo exit=$?
    python3 -I scripts/qa/mp1_plants.py --all --native-only; echo exit=$?
    python3 -I scripts/qa/mp1_plants.py --list --all          # select, run nothing

Exit codes: 0 all selected plants are red and restored (or none is selected); 1 a plant failed;
2 a record is refused; 130 or 143 interrupted. `--files` selects the records that have the path in
`files[].file`. `--timeout S` bounds each command run (default 1200 s). `--out P` writes the results
as JSON. `--root` and `--plants-dir` select another worktree or folder (used by the self-test).

## Forward merges

A forward merge can stop a record from applying (plan section 1, qa N5). The owner of the merge
re-pins the `edit` or `patch` text of the record and runs the plants of the merged paths red in the
same merge. Update `commit` when you re-pin.

The parallel runner (`parallel_refutations_v2.py`, set `mp1`) runs each native record in a scratch
copy of the worktree, with its own cargo target. It does not run `headed: true` records: they run
through the wrapper in F-HEAD and in REG2.
