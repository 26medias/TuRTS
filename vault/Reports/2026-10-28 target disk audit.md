# Target disk audit — 2026-10-28

Read-only audit after M3 integration. target consumes approximately433GiB (du -h). Filesystem has101GiB available and is95% full. No cleanup performed.

Largest top-level areas: debug100G, m2-protocol83G, m2-public-proof47G, m3-evidence35G, m2-export35G, m3-t1632G, m3-integration22G, m3-c420G, m3-c4-causal14G, m3-worktrees12G, m3-final-probe9.4G. Figures are rounded binary units and nested entries below must not be added twice.

Most usage is compiled dependencies and duplicate build caches, not source files. debug/deps alone91G. m3-worktrees/t16/target accounts for11G of its12G total. m3-evidence/t15b-review contains two14G native-target caches (28G total) mixed into evidence. m2-public-proof also mixes real proof records with31G debug output and7.6G mutation builds. Evidence folders must not be deleted wholesale without separating logs/manifests/screenshots from rebuildable targets.

There are24 registered M3 worktrees inside target/m3-worktrees; all24 HEAD commits are ancestors of main.22 have clean tracked/untracked git status. t0 has12 untracked evidence logs. t15b-mutant has12 modified/untracked paths from test instrumentation/mutations; archive its diff and untracked files before removal. Git status does not include ignored files, so inspect/archive meaningful ignored private ledgers before deleting even a clean worktree. Old M2 worktrees outside the repository are also registered but require their own audit and are outside this cleanup scope.

No cargo/rustc/trunk build is running. The active preview is self-contained at target/m3-playtest/20261028-1354e33: server2846901, desktop controller2847108. Preserve this if retaining the demo. Existing main source and Git history do not require the old task worktrees.

Recommended cleanup: preserve durable proof logs/manifests/screenshots and archive the dirty worktree material; retain one current build cache (m3-integration/m3-t0) and the active self-contained demo; delete obsolete build caches with verified path/process ownership; remove retired checkouts with git worktree remove after archival, without force-discarding unique files. Likely reclaimable well over300GiB, subject to per-path archival and active-use checks. Keep target/evidence retention explicit after future milestone merges. Parallel isolation and fault-test cache separation were justified during execution; retaining all caches/checkouts after integration is unnecessary.

## Cleanup completed

User authorized removal. Removed93 obsolete Cargo build-profile directories inside target, plus11 in external TuRTS worktrees after active-use checks. Removed24 registered retired M3 worktrees and one stale unregistered scratch copy. All unique changes, untracked logs and private notes were preserved and archive-verified before deletion; unchanged source remains in Git. Compressed retained scratch material totals about573MiB. Archives, patches, worktree HEAD/status metadata and deletion receipts: target/cleanup-archive/20261028/.

The11 external M2 source checkouts were retained because they have unmerged history or unique changes; their Git statuses were verified unchanged after clearing build output. Nothing outside these explicitly verified TuRTS build-cache paths was deleted. No branch histories or remote refs were deleted.

Preserved the current build cache and moved its profiles into standard target/debug, target/release and target/wasm32-unknown-unknown so ordinary Cargo builds can reuse it. Retained29,939 M3 evidence files, checked for unchanged size/timestamp after deletion. Remaining old-named folders mostly hold logs/results rather than build caches. Historical evidence source-worktree paths are retired; recover source from recorded Git HEADs or retained scratch archives.

The active demo's small Playwright/runtime helper dependencies were copied into its self-contained preview directory and its launcher updated and syntax/import checked before deleting its former worktree dependency. Demo remains running at7140/7141, independently checked HTTP200. Existing local-play artifacts retained. No source/build behavior changed and no new Cargo test run was needed for artifact removal.

Measured result: target went from roughly433GiB to34.1GiB; total filesystem space freed including external caches is425.6GiB. Free space is now526.5GiB. Main source HEAD remains5461486. This report is local vault documentation; no push was performed for cleanup.
