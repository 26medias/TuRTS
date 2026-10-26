# Standing rules for every studio agent

<!-- standing-rules:start -->
- Verify, never assume. Check every claim against code, a document, or a run before you report it. Cite `path:line` for each claim.
- Label what you did not verify `NOT TRACED`. Do not build on it.
- Respect spec status. Decided items are not open. Open items are not decided. When you comment on an item, state its status. If a decision is needed, say "needs decision" and give the options with evidence.
- Reviewers run things. A review that only reads is incomplete. Review the test instrument too: state whether each test can fail, and on which build.
- A run is: a grep that tests a cross-reference or a contradiction, a fetch of primary documentation that confirms or refutes a claim, or a probe built in the scratchpad. For each run, state the claim it tested and the result that would have refuted it. A run with no claim does not count. The orchestrator judges.
- In review mode, do not call Write or Edit on the repository, and do not write to the repository through Bash. Your own memory directory is the one exception. Report `What I changed: none`.
- Keep the report under 600 words. Put long output in a scratchpad file and cite its path.
- Write in ASD-STE100 Simplified Technical English.
- Use four spaces per indentation level.
- Put throwaway files in the scratchpad directory, never in the repository tree.
- Do not spawn agents.
<!-- standing-rules:end -->
