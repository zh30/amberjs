#!/usr/bin/env python3
"""Fix missing imports in Amber source files"""

import os
import re

# Files missing tracing import
tracing_files = [
    "src/ai_inference/pytorch_engine.rs",
    "src/enterprise/k8s_manager.rs",
    "src/enterprise/container_manager.rs",
    "src/enterprise/logging/log_aggregator.rs",
    "src/observability/metrics.rs",  # Already fixed
    "src/ai/ai_performance_engine.rs",
    "src/cloud/auto_scaling.rs",
]

# Files missing serde import
serde_files = [
    "src/enterprise/tenancy/manager.rs",
    "src/enterprise/k8s/operator.rs",
    "src/enterprise/monitoring/metrics.rs",
    "src/observability/visualization/graphs.rs",
    "src/testing/assertions.rs",
    "src/testing/snapshot/mod.rs",
    "src/testing/perf/regression_detector.rs",
]

def fix_tracing_import(filepath):
    """Add tracing import if missing"""
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # Find the last use statement before the first struct/impl
    insert_idx = 0
    for i, line in enumerate(lines):
        if line.startswith('use '):
            insert_idx = i + 1

    # Check if already has tracing import
    if any('use tracing::' in line for line in lines[:insert_idx]):
        print(f"  {filepath} already has tracing import")
        return False

    # Add tracing import
    lines.insert(insert_idx, "use tracing::{debug, info, warn, error};\n")
    with open(filepath, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    print(f"  Fixed tracing import in {filepath}")
    return True

def fix_serde_import(filepath):
    """Add serde import if missing"""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Check if already has serde import
    if 'use serde::' in content:
        print(f"  {filepath} already has serde import")
        return False

    # Find position to insert (after other std:: imports)
    match = re.search(r'(use std::[^;]+;\n)+', content)
    if match:
        insert_pos = match.end()
        new_content = content[:insert_pos] + "use serde::{Serialize, Deserialize};\n" + content[insert_pos:]
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"  Fixed serde import in {filepath}")
        return True

    return False

print("Fixing missing tracing imports...")
for filepath in tracing_files:
    if os.path.exists(filepath):
        fix_tracing_import(filepath)
    else:
        print(f"  File not found: {filepath}")

print("\nFixing missing serde imports...")
for filepath in serde_files:
    if os.path.exists(filepath):
        fix_serde_import(filepath)
    else:
        print(f"  File not found: {filepath}")

print("\nDone!")
