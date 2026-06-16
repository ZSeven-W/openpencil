#!/usr/bin/env python3
"""ab-v9 — element-manifest arm matrix over the ab-v3 corpus (Rust pipeline).

Spec: openpencil-docs/superpowers/specs/2026-06-10-element-manifest-v2.md §2.
Runs the FULL Rust orchestrator (op-smoke headless, `OPENPENCIL_MANIFEST=1`)
for every corpus prompt × provider, then scores M3 (expected-shape: required
roles present in the saved .op tree) + M5 (semantic element selection,
version-suffix-stripped) like the TS harness's score-run.ts.

Keys come from env ONLY (never hardcoded — Codex review of the 06-04
benchmark scripts): MM_KEY / ARK_KEY / DS_KEY / GLM_KEY. Rows append to
scores.jsonl as each cell finishes (crash recovery — lesson from ab-corpus
2cd7a6b2).

Usage:
  MM_KEY=... ARK_KEY=... DS_KEY=... GLM_KEY=... python3 scripts/ab-v9/run_matrix.py \
      [--out /tmp/ab-v9] [--workers 6] [--models minimax,ark,deepseek] \
      [--only prompt-id,...]
  python3 scripts/ab-v9/run_matrix.py --score-only --out /tmp/ab-v9
"""

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CORPUS = os.path.join(REPO, "packages/pen-ai-skills/corpus/ab-v3")
SMOKE = os.path.join(REPO, "target/debug/op-smoke")

ARK_BASE = "https://ark.cn-beijing.volces.com/api/coding/v3"
# GLM 原生 Coding Plan(open.bigmodel.cn/api/coding/paas/v4,CN)。key 是
# 智谱 `id.secret` 对(`xxx.yyy`),整串当 Bearer 下发,GLM 自己拆。区别于
# 方舟 CP 的 ark-glm-5.1:这条直连智谱官方 CP。
GLM_BASE = "https://open.bigmodel.cn/api/coding/paas/v4"
PROVIDERS = {
    # 用户指定:MiniMax 用最新 M3(关思考由 DirectClient 的 is_minimax_model
    # 自动下发);DS 官方端点没有 v4.1,v4-pro 即最新 pro;方舟挑主流最新
    # (该账号无 GLM5.1):GLM-4.7 + Kimi-K2-thinking,另保留 ark-code-latest
    # 作为方舟基线。
    "minimax-m3": {
        "base": "https://api.minimaxi.com/v1",
        "model": "MiniMax-M3",
        "key_env": "MM_KEY",
    },
    "minimax-m2.7": {
        "base": "https://api.minimaxi.com/v1",
        "model": "MiniMax-M2.7",
        "key_env": "MM_KEY",
    },
    # M3 开思考臂:nothink 摆烂(ab-v9 17%),开思考是 M3 线唯一候选。
    "minimax-m3-think": {
        "base": "https://api.minimaxi.com/v1",
        "model": "MiniMax-M3",
        "key_env": "MM_KEY",
        "extra_env": {"OPENPENCIL_SMOKE_KEEP_THINKING": "1"},
    },
    "deepseek-v4-pro": {
        "base": "https://api.deepseek.com/v1",
        "model": "deepseek-v4-pro",
        "key_env": "DS_KEY",
    },
    "ark-code": {
        "base": ARK_BASE,
        "model": "ark-code-latest",
        "key_env": "ARK_KEY",
    },
    # glm-5.1 是 coding-plan 隐藏别名(不在 /models 列表,实测可调,与 TS
    # ab-corpus 2026-04-22 起的路由一致)。kimi-k2.6 同为别名但 2026-06-10
    # 起按用户要求移除——方舟主流模型一次只允许启用一个,账号当前启用的
    # 是 GLM-5.1。
    "ark-glm-5.1": {
        "base": ARK_BASE,
        "model": "glm-5.1",
        "key_env": "ARK_KEY",
    },
    # 智谱官方 Coding Plan,GLM-5.2(用户 2026-06-15 提供的 id.secret key)。
    "glm-5.2": {
        "base": GLM_BASE,
        "model": "glm-5.2",
        "key_env": "GLM_KEY",
    },
}

CELL_TIMEOUT_S = 1200


def parse_corpus_yaml(path):
    """Constrained parser for the corpus schema (no pyyaml on this host).

    Handles: top-level scalars, `prompt: |` block, `expected:` with
    `must_contain_roles` list + `min_roles` map.
    """
    entry = {"must_contain_roles": [], "min_roles": {}}
    lines = open(path, encoding="utf-8").read().splitlines()
    i, n = 0, len(lines)
    section = None
    while i < n:
        line = lines[i]
        if line.startswith("prompt: |"):
            block = []
            i += 1
            while i < n and (lines[i].startswith("  ") or lines[i] == ""):
                block.append(lines[i][2:])
                i += 1
            entry["prompt"] = "\n".join(block).strip()
            continue
        if line.startswith("expected:"):
            section = "expected"
            i += 1
            continue
        if section == "expected":
            stripped = line.strip()
            if line.startswith("  must_contain_roles:"):
                sub = "roles"
            elif line.startswith("  min_roles:"):
                sub = "min"
            elif line.startswith("    - "):
                entry["must_contain_roles"].append(stripped[2:].strip())
            elif line.startswith("    ") and ":" in stripped:
                k, v = stripped.split(":", 1)
                try:
                    entry["min_roles"][k.strip()] = int(v.strip())
                except ValueError:
                    pass
            elif line and not line.startswith(" "):
                section = None
                continue
            i += 1
            continue
        m = re.match(r"^([a-z_]+):\s*(.*)$", line)
        if m:
            entry[m.group(1)] = m.group(2).strip()
        i += 1
    return entry if entry.get("id") and entry.get("prompt") else None


def load_corpus(only=None):
    prompts = []
    for name in sorted(os.listdir(CORPUS)):
        if not name.endswith(".yaml"):
            continue
        entry = parse_corpus_yaml(os.path.join(CORPUS, name))
        if entry and (not only or entry["id"] in only):
            prompts.append(entry)
    return prompts


def walk_roles(node, counter):
    if isinstance(node, dict):
        role = node.get("role")
        if isinstance(role, str) and role:
            counter[role] += 1
        for key in ("children", "pages"):
            for child in node.get(key) or []:
                walk_roles(child, counter)
    elif isinstance(node, list):
        for item in node:
            walk_roles(item, counter)


def count_nodes(node):
    if isinstance(node, dict):
        n = 1 if node.get("type") else 0
        for key in ("children", "pages"):
            for child in node.get(key) or []:
                n += count_nodes(child)
        return n
    if isinstance(node, list):
        return sum(count_nodes(item) for item in node)
    return 0


def base_kind(tool):
    base = tool.strip().lower().replace("-", "_")
    base = re.sub(r"^add_", "", base)
    return re.sub(r"_v\d+$", "", base)


def score_cell(entry, op_path, log_path, rc, duration):
    row = {
        "prompt_id": entry["id"],
        "category": entry.get("category", ""),
        "difficulty": entry.get("difficulty", "obvious"),
        "duration_s": round(duration, 1),
        "rc": rc,
    }
    log_text = ""
    if os.path.exists(log_path):
        log_text = open(log_path, encoding="utf-8", errors="replace").read()
    # Reasoning drafts mention element kinds the final answer may not use —
    # strip <think> blocks before counting (mirrors parse.rs strip_reasoning).
    log_text = re.sub(r"<think(?:ing)?>.*?</think(?:ing)?>", "", log_text, flags=re.S)
    row["el_lines"] = len(re.findall(r'\{"el"\s*:', log_text))
    row["warnings"] = dict(Counter(re.findall(r"\[manifest\] (W-[A-Z-]+)", log_text)))
    sys_lens = [int(x) for x in re.findall(r"system_len=(\d+)", log_text)]
    row["max_system_chars"] = max(sys_lens) if sys_lens else 0

    # M5 — semantic element selection (suffix-stripped), only for prompts
    # that name an expected tool.
    expected_tool = entry.get("expected_tool_if_any", "")
    if expected_tool:
        used = {base_kind(k) for k in re.findall(r'\{"el"\s*:\s*"([a-z_0-9-]+)"', log_text)}
        row["m5_expected"] = base_kind(expected_tool)
        row["m5_success"] = row["m5_expected"] in used

    if rc != 0 or not os.path.exists(op_path):
        row["m3_success"] = False
        row["m3_failure_reason"] = f"generation failed (rc={rc})"
        row["garbage"] = True
        return row
    try:
        doc = json.load(open(op_path, encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as err:
        row["m3_success"] = False
        row["m3_failure_reason"] = f"unreadable .op: {err}"
        row["garbage"] = True
        return row

    row["node_count"] = count_nodes(doc)
    row["garbage"] = row["node_count"] <= 1
    roles = Counter()
    walk_roles(doc, roles)
    missing = [r for r in entry["must_contain_roles"] if roles.get(r, 0) == 0]
    below = [
        f"{r} ({roles.get(r, 0)}/{want})"
        for r, want in entry["min_roles"].items()
        if roles.get(r, 0) < want
    ]
    if missing:
        row["m3_success"] = False
        row["m3_failure_reason"] = "missing required role(s): " + ", ".join(missing)
    elif below:
        row["m3_success"] = False
        row["m3_failure_reason"] = "role counts below minimum: " + ", ".join(below)
    else:
        row["m3_success"] = True
        row["m3_failure_reason"] = ""
    return row


def run_cell(entry, provider_id, cfg, out_dir, write_row):
    cell = f"{provider_id}/{entry['id']}"
    op_path = os.path.join(out_dir, "op", provider_id, f"{entry['id']}.op")
    log_path = os.path.join(out_dir, "logs", provider_id, f"{entry['id']}.log")
    os.makedirs(os.path.dirname(op_path), exist_ok=True)
    os.makedirs(os.path.dirname(log_path), exist_ok=True)
    env = dict(
        os.environ,
        OPENPENCIL_MANIFEST="1",
        OPENPENCIL_SMOKE_DIRECT="1",
        OPENPENCIL_SMOKE_DUMP="1",
        OPENPENCIL_LLM_PROVIDER="openai-compat",
        OPENPENCIL_LLM_API_KEY=os.environ[cfg["key_env"]],
        OPENPENCIL_LLM_BASE_URL=cfg["base"],
        OPENPENCIL_ORCHESTRATOR_MODEL=cfg["model"],
        OPENPENCIL_SMOKE_OUT=op_path,
        **cfg.get("extra_env", {}),
    )
    start = time.time()
    try:
        with open(log_path, "w", encoding="utf-8") as log:
            rc = subprocess.run(
                [SMOKE, entry["prompt"]],
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=CELL_TIMEOUT_S,
            ).returncode
    except subprocess.TimeoutExpired:
        rc = -9
    duration = time.time() - start
    row = score_cell(entry, op_path, log_path, rc, duration)
    row["model"] = provider_id
    write_row(row)
    status = "ok" if row["m3_success"] else f"M3-FAIL ({row['m3_failure_reason'][:60]})"
    print(f"[{cell}] {duration:.0f}s {status}", flush=True)


def aggregate(rows):
    out = ["# ab-v9 — manifest arm × ab-v3 corpus\n"]
    out.append(
        "| model | n | M3 | M3 rate | composite M3 | garbage | M5 | avg s |"
    )
    out.append("|---|---|---|---|---|---|---|---|")
    for model in sorted({r["model"] for r in rows}):
        sub = [r for r in rows if r["model"] == model]
        m3 = sum(1 for r in sub if r["m3_success"])
        comp = [r for r in sub if r["difficulty"] == "composite"]
        comp_m3 = sum(1 for r in comp if r["m3_success"])
        garbage = sum(1 for r in sub if r.get("garbage"))
        m5_rows = [r for r in sub if "m5_success" in r]
        m5 = sum(1 for r in m5_rows if r["m5_success"])
        avg = sum(r["duration_s"] for r in sub) / max(len(sub), 1)
        out.append(
            f"| {model} | {len(sub)} | {m3}/{len(sub)} | {m3 / max(len(sub), 1):.0%} "
            f"| {comp_m3}/{len(comp)} | {garbage} | {m5}/{len(m5_rows)} | {avg:.0f} |"
        )
    out.append("\n## 失败归因 top\n")
    reasons = Counter(
        r["m3_failure_reason"].split(":")[0] for r in rows if not r["m3_success"]
    )
    for reason, count in reasons.most_common(8):
        out.append(f"- {count} × {reason}")
    out.append("\n## warning 直方图\n")
    warnings = Counter()
    for r in rows:
        warnings.update(r.get("warnings", {}))
    for w, count in warnings.most_common():
        out.append(f"- {count} × {w}")
    return "\n".join(out) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="/tmp/ab-v9")
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--models", default=",".join(PROVIDERS))
    ap.add_argument("--only", default="")
    ap.add_argument("--score-only", action="store_true")
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    scores_path = os.path.join(args.out, "scores.jsonl")
    only = set(filter(None, args.only.split(",")))
    prompts = load_corpus(only or None)
    models = [m for m in args.models.split(",") if m in PROVIDERS]

    if args.score_only:
        rows = []
        for entry in prompts:
            for model in models:
                op_path = os.path.join(args.out, "op", model, f"{entry['id']}.op")
                log_path = os.path.join(args.out, "logs", model, f"{entry['id']}.log")
                row = score_cell(entry, op_path, log_path, 0, 0.0)
                row["model"] = model
                rows.append(row)
        open(scores_path, "w", encoding="utf-8").write(
            "\n".join(json.dumps(r, ensure_ascii=False) for r in rows) + "\n"
        )
    else:
        for model in models:
            if not os.environ.get(PROVIDERS[model]["key_env"]):
                sys.exit(f"error: {PROVIDERS[model]['key_env']} not set for {model}")
        lock = threading.Lock()
        done = set()
        if os.path.exists(scores_path):  # resume: skip finished cells
            for line in open(scores_path, encoding="utf-8"):
                try:
                    row = json.loads(line)
                    done.add((row["model"], row["prompt_id"]))
                except json.JSONDecodeError:
                    pass

        def write_row(row):
            with lock:
                with open(scores_path, "a", encoding="utf-8") as f:
                    f.write(json.dumps(row, ensure_ascii=False) + "\n")

        cells = [
            (entry, model)
            for entry in prompts
            for model in models
            if (model, entry["id"]) not in done
        ]
        print(f"[ab-v9] {len(cells)} cells ({len(prompts)} prompts × {models})", flush=True)
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            for entry, model in cells:
                pool.submit(run_cell, entry, model, PROVIDERS[model], args.out, write_row)

    rows = [json.loads(line) for line in open(scores_path, encoding="utf-8")]
    report = aggregate(rows)
    open(os.path.join(args.out, "report.md"), "w", encoding="utf-8").write(report)
    print(report)


if __name__ == "__main__":
    main()
