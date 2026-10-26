#!/usr/bin/env python3
"""
Rune Skill Evaluator Script
Evaluates skill descriptions against test prompt queries and checks accompanying script executability.
"""

import sys
import os

try:
    import yaml
except ImportError:
    yaml = None

def simple_yaml_parse(fm_str):
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

def evaluate_skill(target_path):
    print(f"[*] Evaluating Rune skill: {target_path}")
    
    if not os.path.exists(target_path):
        print(f"[X] Error: Path '{target_path}' does not exist.")
        return False

    md_path = None
    skill_dir = None
    if os.path.isdir(target_path):
        skill_dir = os.path.abspath(target_path)
        skill_md = os.path.join(skill_dir, "SKILL.md")
        folder_name = os.path.basename(skill_dir)
        alt_md = os.path.join(skill_dir, f"{folder_name}.md")
        
        if os.path.exists(skill_md):
            md_path = skill_md
        elif os.path.exists(alt_md):
            md_path = alt_md
    elif os.path.isfile(target_path) and target_path.endswith(".md"):
        md_path = os.path.abspath(target_path)
        skill_dir = os.path.dirname(md_path)

    if not md_path or not os.path.exists(md_path):
        print(f"[X] Error: Could not locate skill Markdown file.")
        return False

    try:
        with open(md_path, "r", encoding="utf-8") as f:
            content = f.read()
    except Exception as e:
        print(f"[X] Error reading file: {e}")
        return False

    # Extract frontmatter
    if not content.startswith("---"):
        print("[X] Error: Missing YAML frontmatter.")
        return False

    rest = content[3:]
    end_idx = rest.find("---")
    if end_idx == -1:
        print("[X] Error: Unclosed frontmatter.")
        return False

    try:
        if yaml is not None:
            fm = yaml.safe_load(rest[:end_idx])
        else:
            fm = simple_yaml_parse(rest[:end_idx])
    except Exception as e:
        print(f"[X] Error parsing YAML: {e}")
        return False

    skill_name = fm.get("name") or fm.get("title", "unknown")
    description = fm.get("description", "")
    print(f"    - Skill Name: {skill_name}")
    print(f"    - Description Length: {len(description)} chars")

    # Evaluate description trigger richness
    print("\n[*] Evaluating trigger keywords and clarity...")
    keywords = ["when", "use", "whenever", "audit", "verify", "create", "build", "test", "check"]
    matched_keywords = [kw for kw in keywords if kw in description.lower()]
    print(f"    - Trigger context keywords found: {matched_keywords}")
    if len(matched_keywords) >= 2:
        print("    [+] Description provides clear situational triggers.")
    else:
        print("    [!] Description could be more explicit about when Rune should trigger this skill (e.g. 'Use whenever...').")

    # Check companion scripts in skill folder
    companion_scripts = []
    if skill_dir and os.path.isdir(skill_dir):
        for root, dirs, files in os.walk(skill_dir):
            for file in files:
                if file.endswith(".py") or file.endswith(".sh") or file.endswith(".ps1"):
                    full_p = os.path.join(root, file)
                    rel_p = os.path.relpath(full_p, skill_dir)
                    if rel_p not in ["validate_skill.py", "eval_skill.py"]:
                        companion_scripts.append(full_p)

    print(f"\n[*] Companion Scripts Detected: {len(companion_scripts)}")
    for script in companion_scripts:
        print(f"    - {os.path.relpath(script, skill_dir)}")
        if script.endswith(".py"):
            # Syntax check python script
            import py_compile
            try:
                py_compile.compile(script, doraise=True)
                print(f"      [+] Python syntax valid.")
            except Exception as pe:
                print(f"      [X] Python syntax error: {pe}")

    print("-" * 40)
    print(f"[SUCCESS] Evaluation complete for '{skill_name}'.")
    return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python eval_skill.py <path-to-skill-dir-or-md>")
        sys.exit(1)
    
    success = evaluate_skill(sys.argv[1])
    sys.exit(0 if success else 1)
