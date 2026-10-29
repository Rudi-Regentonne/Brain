# Zed Extension for Brain (`.brn`)

Language support for Brain in **Zed**.

---

## Quick Setup in Zed (Local Dev Extension)

Zed supports loading local extensions directly:

1. Open Zed.
2. Open the command palette (`Ctrl+Shift+P` on Linux/Windows, `Cmd+Shift+P` on macOS).
3. Type and select **`zed: install dev extension`**.
4. Choose the `editors/zed` directory from this repository:
   ```text
   /path/to/sebastian/editors/zed
   ```
5. Done! All `.brn` files will now be recognized with full syntax highlighting, bracket auto-closing, and comment toggles in Zed.

---

## Alternative: User Language Mapping (No Extension Needed)

If you don't want to install an extension, you can simply map `.brn` files in your Zed `settings.json` (`Ctrl+,` or `Cmd+,`):

```json
{
  "file_types": {
    "C#": ["brn"]
  }
}
```
Because Brain syntax aligns with C#-style tokens and attributes (`[...]`, classes, methods, braces), Zed's built-in Tree-sitter engine will highlight `.brn` files immediately!
