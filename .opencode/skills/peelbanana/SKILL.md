---
name: peelbanana
description: "Helper for peeling a banana"
triggers:
  - "peelbanana"
platform: opencode
---

# Peelbanana

Helper for peeling a banana

<prerequisites>
- Requires that Coding agent has a banana and Banana is ripe
- Repeat while Basket is not empty:
-   - wait until Basket is full of bananas
</prerequisites>

<parameters>
- `fast` (`bool`) (default: `false`): Parameter `fast`
</parameters>

<instructions workflow="Run">
1. **WHEN** banana is ripe:
  - Output: "Pinch the bottom tip and pull peels backwards."
  **OTHERWISE**:
  - Output: "Wait a few days until banana turns yellow."
</instructions>

