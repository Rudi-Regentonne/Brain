# 📁 Brain Skill Examples Directory

This directory showcases real-world examples written in Brain (`.brn`).

---

## 1. Minimal Skill: Banana Peeler
- **Location:** `.brains/peelbanana/PeelbananaSkill.brn`
- **Features:**
  - Simple conditional branches (`if / else`)
  - Human-friendly input parameters
  - Preconditions check

```brain
[Skill("peelbanana", "Helper for peeling a banana")]
[Trigger("peelbanana")]
public class PeelbananaSkill
{
    public void Run(bool fast = false)
    {
        if ("banana is ripe")
        {
            Prompt("Pinch the bottom tip and pull peels backwards.");
        }
        else
        {
            Prompt("Wait a few days until banana turns yellow.");
        }
    }
}
```

---

## 2. Advanced Multi-File Skill: Ultimate Assistant
- **Main Skill:** `.brains/ultimate-assistant/UltimateAssistantSkill.brn`
- **Local Module:** `.brains/ultimate-assistant/secondfile.brn`
- **Shared Module:** `.brains/shared/SharedGitStandards.brn`
- **Supporting Assets:**
  - `.brains/ultimate-assistant/scripts/lint.py` (executable Python script)
  - `.brains/ultimate-assistant/schemas/report.schema.json` (JSON schema output)
- **Features Demonstrated:**
  - Modular `using` imports (both sibling files and shared libraries)
  - Distinguishing tool execution (`Execute(...)`) from prompts (`Prompt(...)`)
  - Hard negative guardrails (`[Never(...)]`) and rules (`[Rule(...)]`)
  - Contrastive few-shot examples (`Good` vs `Bad`)
  - `try / catch` fallback instructions
  - Automatic synchronization of assets (`scripts/`, `schemas/`) into target harness folders
