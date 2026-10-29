# 🧠 Brain Language Reference & Guide

Complete language specification, attributes, and usage guide for **Brain (`.brn`)** files.

---

## 1. File & Directory Layout

Brain organizes skills in a dedicated `.brains/` directory:

```text
my-project/
├── .brains/
│   ├── brain.toml                 # Project configuration
│   ├── shared/                    # Reusable modules (no [Skill] attribute)
│   │   └── SharedRules.brn
│   ├── git-commit/                # Skill subfolder
│   │   ├── GitCommitSkill.brn     # Skill definition with [Skill]
│   │   ├── helpers.brn            # Supporting .brn module
│   │   ├── scripts/               # Helper scripts (auto-synced)
│   │   │   └── validate.py
│   │   └── schemas/               # Schemas or templates (auto-synced)
│   │       └── output.json
└── .opencode/skills/              # Compiled output for OpenCode
    └── git-commit/
        ├── .brainhash             # Companion hash (zero token bloat)
        ├── SKILL.md               # Compiled prompt instructions
        ├── scripts/
        └── schemas/
```

---

## 2. Configuration (`.brains/brain.toml`)

```toml
[project]
name = "my-ai-skills"

[compiler]
# Targets to build for: "opencode", "claude"
targets = ["opencode", "claude"]
# Keep SKILL.md clean with zero comment overhead
include_watermark = false

[safety]
# Protects non-Brain hand-crafted skills
overwrite_foreign_skills = false
```

---

## 3. Brain Attributes

| Attribute | Description | Example |
| :--- | :--- | :--- |
| `[Skill("name", "desc")]` | Declares an executable top-level skill | `[Skill("code-review", "Reviews PR code changes")]` |
| `[Trigger("...", "...")]` | Keywords or invocation triggers | `[Trigger("review code", "pr check")]` |
| `[Requires("...", "...")]` | Tools required by the agent | `[Requires("git", "shell", "read-filesystem")]` |
| `[Guardrail("...")]` / `[Never("...")]` | Hard negative constraints (compiled to `<guardrails>`) | `[Never("commit secrets or .env files")]` |
| `[Rule("...")]` | Operational guidelines (compiled to `<rules>`) | `[Rule("Keep explanations concise and technical")]` |
| `[Precondition("...")]` | Prerequisites check before skill execution | `[Precondition("A git repository must be active")]` |
| `[Example(...)]` | Few-shot demonstrations with contrastive pairs | `[Example(Input = "...", Bad = "...", Good = "...")]` |

---

## 4. Control Flow & Language Constructs

### Natural Language Conditionals (`if / else`)
Conditions in `if` and `while` can be typed booleans or natural language strings:

```brain
if ("banana is ripe")
{
    Prompt("Pinch the bottom tip and pull peels backwards.");
}
else
{
    Prompt("Wait a few days until banana turns yellow.");
}
```
*Compiles to state-anchored decision trees (`**WHEN**` / `**OTHERWISE**`).*

### Tool Commands (`Execute`) vs Prompts (`Prompt`)
- **`Execute("command")`**: Tells the agent to run an autonomous shell tool call.
  ```brain
  Execute("python3 scripts/lint.py");
  // Compiles to: Run command in shell: `python3 scripts/lint.py`
  ```
- **`Prompt("message")`**: Directs the agent what to report or explain to the user.
  ```brain
  Prompt("Summarizing changes for user.");
  // Compiles to: Output: "Summarizing changes for user."
  ```

### Bounded Loops
Avoid open-ended infinite loops by bounding with parameters:
```brain
while ($"There are unexamined diff files (processed < {maxFiles})")
{
    Prompt("Inspect next diff chunk.");
}
```

### Try / Catch Error Fallbacks
```brain
try
{
    Execute("git diff --staged");
}
catch (DiffTooLargeException)
{
    Prompt("Diff exceeds threshold: falling back to status summary.");
    Execute("git status -s");
}
```

---

## 5. Module Imports (`using`)

- **Relative paths**:
  ```brain
  using "../shared/SharedGitStandards.brn";
  using "./secondfile.brn";
  ```
- **Module notation**:
  ```brain
  using Shared.CommonRules;
  ```

*Files without a `[Skill]` attribute are treated as shared library modules. Their procedures and rules are composed into the consuming skill, without creating an empty standalone skill.*
