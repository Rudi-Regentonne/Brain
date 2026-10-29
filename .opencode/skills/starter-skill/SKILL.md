---
name: starter-skill
description: "Example starter AI skill written in Brain"
triggers:
  - "run starter"
  - "starter"
tools:
  - "shell"
platform: opencode
---

# Starter Skill

Example starter AI skill written in Brain

<rules>
- Always confirm actions before executing dangerous commands
</rules>

<prerequisites>
- Check prerequisites via CheckPrerequisites
</prerequisites>

<parameters>
- `task` (`string`) (default: `"greeting"`): Parameter `task`
- `verbose` (`bool`) (default: `false`): Parameter `verbose`
</parameters>

<instructions workflow="Run">
1. **WHEN** verbose:
  - Output: "Starting starter skill with task: {task}"
2. Perform primary action:
  - Execute `PerformTask(task)`
  **On ExecutionError fallback**:
  - Output: "Encountered an issue executing task, falling back to safe status check."
</instructions>

<procedure name="PerformTask">
1. Output: "Execute action for task: {task}"
</procedure>

