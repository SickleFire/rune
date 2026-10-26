#!/usr/bin/env python3
"""
Rune Skill Validator Script
Validates SKILL.md or standalone .md skill files for Rune structure, frontmatter schemas, and naming conventions.
"""

import sys
import os
import re

# Fallback YAML parser if PyYAML is not installed
try:
    import yaml
except ImportError:
    yaml = None

def simple_yaml_parse(fm_str):
    """Basic parser for simple key-value YAML frontmatter if PyYAML is absent."""
    data = {}
    for line in fm_str.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" in line:
            parts = line.split(":", 1)
            key = parts[0].strip()
            val = parts[1].strip().strip('"\'')
            data[key] = val
    return data

def validate_skill(target_path):
    print(f"[*] Validating Rune skill path: {target_path}")
    
    if not os.path.exists(target_path):
        print(f"[X] Error: Path '{target_path}' does not exist.")
        return False

    md_path = None
    if os.path.isdir(target_path):
        skill_md = os.path.join(target_path, "SKILL.md")
        folder_name = os.path.basename(os.path.abspath(target_path))
        alt_md = os.path.join(target_path, f"{folder_name}.md")
        
        if os.path.exists(skill_md):
            md_path = skill_md
        elif os.path.exists(alt_md):
            md_path = alt_md
        else:
            print(f"[X] Error: Directory '{target_path}' must contain either 'SKILL.md' or '<folder_name>.md'.")
            return False
    elif os.path.isfile(target_path) and target_path.endswith(".md"):
        md_path = target_path
    else:
        print(f"[X] Error: Target must be a directory containing SKILL.md or a Markdown file.")
        return False

    print(f"[+] Found skill markdown: {md_path}")
    
    try:
        with open(md_path, "r", encoding="utf-8") as f:
            content = f.read()
    except Exception as e:
        print(f"[X] Error reading file: {e}")
        return False

    # Check Frontmatter
    if not content.startswith("---"):
        print("[X] Error: Skill file must start with YAML frontmatter delimited by '---'.")
        return False

    rest = content[3:]
    end_idx = rest.find("---")
    if end_idx == -1:
        print("[X] Error: Unclosed YAML frontmatter (missing closing '---').")
        return False

    fm_str = rest[:end_idx]
    try:
        if yaml is not None:
            fm = yaml.safe_load(fm_str)
        else:
            fm = simple_yaml_parse(fm_str)
    except Exception as ye:
        print(f"[X] Error parsing YAML frontmatter: {ye}")
        return False

    if not isinstance(fm, dict):
        print("[X] Error: Frontmatter must be a valid YAML dictionary.")
        return False

    errors = 0
    warnings = 0

    # Required fields
    if "name" not in fm and "title" not in fm:
        print("[X] Error: Frontmatter is missing required 'name' (or 'title') field.")
        errors += 1
    else:
        name = fm.get("name") or fm.get("title")
        print(f"    - name: {name}")
        if isinstance(name, str) and not re.match(r"^[a-z0-9\-]+$", name.lower().replace("skill: ", "").replace(" ", "-")):
            print(f"[!] Warning: Skill name '{name}' should preferably be kebab-case.")
            warnings += 1

    if "description" not in fm:
        print("[X] Error: Frontmatter is missing required 'description' field.")
        errors += 1
    else:
        desc = fm["description"]
        print(f"    - description: {desc[:60]}...")
        if len(desc) < 20:
            print("[!] Warning: Description is very short. Make sure it describes clear contextual triggers for Rune.")
            warnings += 1

    # Check Markdown body
    body = rest[end_idx+3:]
    if "# " not in body:
        print("[!] Warning: Markdown body lacks a top-level header (# Title).")
        warnings += 1

    if "Workflow" not in body and "workflow" not in body.lower() and "when invoked" not in body.lower() and "when tasked" not in body.lower():
        print("[!] Warning: Skill instructions lack a 'Workflow' section or step-by-step guide.")
        warnings += 1

    print("-" * 40)
    if errors == 0:
        print(f"[SUCCESS] Skill validation passed with {warnings} warning(s).")
        return True
    else:
        print(f"[FAIL] Skill validation failed with {errors} error(s) and {warnings} warning(s).")
        return False

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python validate_skill.py <path-to-skill-dir-or-md>")
        sys.exit(1)
    
    success = validate_skill(sys.argv[1])
    sys.exit(0 if success else 1)
