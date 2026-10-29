---
name: ultimate-assistant
description: "Comprehensive AI skill demonstrating all Brain features, tools, commands, scripts, and rules"
triggers:
  - "run assistant"
  - "assistant"
  - "review code"
tools:
  - "git"
  - "shell"
  - "read-filesystem"
platform: opencode
---

# Ultimate Assistant

Comprehensive AI skill demonstrating all Brain features, tools, commands, scripts, and rules

<guardrails>
CRITICAL:
- DO NOT execute `git push --force` or delete untracked files without confirmation
- DO NOT commit secrets, private keys, or `.env` credential files
</guardrails>

<rules>
- Keep explanations concise and technical
- Ensure all code changes adhere to project formatting standards
- Format commit messages according to Conventional Commits
- Review must analyze diff chunks with maxFiles bound
</rules>

<prerequisites>
- Requires that Git repository is initialized and Workspace has changes to inspect
</prerequisites>

<parameters>
- `scope` (`string`) (default: `"core"`): Parameter `scope`
- `verbose` (`bool`) (default: `false`): Parameter `verbose`
- `maxFiles` (`int`) (default: `10`): Parameter `maxFiles`
</parameters>

<instructions workflow="Run">
1. **WHEN** verbose:
  - Output: "Starting review session for scope: {scope} with limit {maxFiles} files."
2. Execute `VerifyGitClean()`
3. Execute `ExecuteLintScript()`
4. Perform primary action:
  - Execute `InspectStagedChanges(scope, maxFiles)`
  - Execute `ReportReviewSummary(scope)`
  - **WHEN** All checks and lint scripts pass:
    - Output: "Draft conventional commit: feat({scope}): summary of changes."
    **OTHERWISE**:
    - Output: "List all failed checks with specific line numbers and fixes."
  **On DiffTooLargeException fallback**:
  - Output: "Diff exceeds context threshold: falling back to file status list."
  - Execute `FallbackFileSummary()`
</instructions>

<procedure name="VerifyGitClean">
1. Run command in shell: `git status`
</procedure>

<procedure name="FallbackFileSummary">
1. Run command in shell: `git status -s`
</procedure>

<procedure name="ExecuteLintScript">
1. Run command in shell: `python3 scripts/lint.py`
</procedure>

<procedure name="InspectStagedChanges">
1. **Repeat while** "There are unexamined diff files (processed < {maxFiles})":
  - Output: "Analyze next modified file under {scope} for security and style compliance."
</procedure>

<procedure name="ReportReviewSummary">
1. Output: "Output formatted review summary for component `{scope}`."
</procedure>

<examples>
- Input: review staged changes with fast mode
  ❌ BAD: Looks fine to me, ship it!
  ✅ GOOD: Inspected 3 files: 2 additions, 0 vulnerabilities. Ran scripts/lint.py with 0 errors. Staging approved.
- Input: commit with broken tests
  ❌ BAD: git commit -m 'wip fix'
  ✅ GOOD: Rejected commit: tests are failing. Run tests before committing.
</examples>

