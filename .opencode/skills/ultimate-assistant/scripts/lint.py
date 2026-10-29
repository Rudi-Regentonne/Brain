#!/usr/bin/env python3
import sys

def lint_diff(diff_content):
    print("Linting staged diff...")
    if "TODO" in diff_content:
        print("[WARN] Found unresolved TODO comments in diff.")
    print("[OK] Lint analysis completed cleanly.")
    return 0

if __name__ == "__main__":
    diff_input = sys.stdin.read() if not sys.stdin.isatty() else ""
    sys.exit(lint_diff(diff_input))
