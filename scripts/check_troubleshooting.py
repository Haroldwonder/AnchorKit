#!/usr/bin/env python3
"""Fail if any ErrorCode variant in src/errors.rs lacks a section in TROUBLESHOOTING.md."""
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
src = (root / "src/errors.rs").read_text()
doc = (root / "docs/guides/TROUBLESHOOTING.md").read_text()

m = re.search(r"pub enum ErrorCode \{(.*?)\n\}", src, re.S)
if not m:
    sys.exit("could not find `pub enum ErrorCode` in src/errors.rs")
variants = dict(re.findall(r"^\s*(\w+)\s*=\s*(\d+)", m.group(1), re.M))

errors = []
for name, num in variants.items():
    sec = re.search(rf"^## {name}\n\n\*\*Code:\*\* (\d+)", doc, re.M)
    if not sec:
        errors.append(f"missing section: ## {name}")
    elif sec.group(1) != num:
        errors.append(f"{name}: doc says code {sec.group(1)}, src/errors.rs says {num}")
documented = set(re.findall(r"^## (\w+)$", doc, re.M))
for extra in sorted(documented - variants.keys()):
    errors.append(f"documented but not in ErrorCode: {extra}")

if errors:
    print("docs/guides/TROUBLESHOOTING.md is out of sync with src/errors.rs:")
    print("\n".join(f"  - {e}" for e in errors))
    sys.exit(1)
print(f"OK: all {len(variants)} ErrorCode variants documented")
