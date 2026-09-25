#!/usr/bin/env python3
"""把 cjkcheck 的报告与 snapshot_layout 的实测行数交叉：
真实行数 == 硬换行数 => 根本没发生自动折行 => 该报告是保守模拟的误报。
只留真正折了行的节点。"""
import json, os, re, shutil, subprocess, sys, pathlib, tempfile
# OP_BIN_DIR points at another profile / worktree (e.g. target/debug).
BIN=str(pathlib.Path(os.environ.get("OP_BIN_DIR",pathlib.Path(__file__).resolve().parents[4]/"target"/"release"))/"openpencil-desktop")
TOOL=str(pathlib.Path(__file__).resolve().parent / "cjkcheck.py")
def snap(path):
    req=[{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},
         {"jsonrpc":"2.0","id":2,"method":"tools/call",
          "params":{"name":"snapshot_layout","arguments":{"depth":40}}}]
    # `--mcp <file>` finalizes and WRITES BACK to the file; run it on a copy.
    with tempfile.TemporaryDirectory() as tmp:
        copy=pathlib.Path(tmp)/pathlib.Path(path).name
        shutil.copy(path,copy)
        out=subprocess.run([BIN,"--mcp",str(copy)],input="\n".join(json.dumps(r) for r in req)+"\n",
                           capture_output=True,text=True).stdout
    last=[l for l in out.strip().splitlines() if l.startswith("{")][-1]
    return json.loads(json.loads(last)["result"]["content"][0]["text"])["layout"]
def flat(ns,acc):
    for n in ns:
        acc[n["id"]]=n; flat(n.get("children") or [],acc)
    return acc
def texts(doc):
    m={}
    def go(n):
        if isinstance(n,dict):
            if n.get("type")=="text":
                m[n["id"]]=(n.get("name"),n.get("content"),n.get("fontSize"),n.get("lineHeight") or 1.4)
            for c in (n.get("children") or []): go(c)
        elif isinstance(n,list):
            for c in n: go(c)
    go(doc.get("children") or []); return m
ARGS=[pathlib.Path(a) for a in sys.argv[1:]]
TARGETS=ARGS or sorted(pathlib.Path("crates/op-editor-core/assets/scene_templates").glob("*.op"))
total_real=0
for p in TARGETS:
    rep=subprocess.run([sys.executable,TOOL,str(p)],capture_output=True,text=True).stdout
    hits=[l for l in rep.splitlines() if l.startswith(("孤字末行","行首标点","疑似超框"))]
    if not hits: continue
    lay=flat(snap(str(p)),{}); m=texts(json.load(open(p)))
    # 名字 -> 是否真的折行
    wrapped={}
    for nid,(nm,content,fs,lh) in m.items():
        n=lay.get(nid)
        if not n or not fs or not isinstance(content,str): continue
        lines=round(n["height"]/(fs*lh)); hard=content.count("\n")+1
        wrapped.setdefault(nm,False)
        if lines>hard: wrapped[nm]=True
    # 名字 -> 真实行数（同名多节点取集合）
    lines_by_name={}
    for nid,(nm,content,fs,lh) in m.items():
        n=lay.get(nid)
        if not n or not fs or not isinstance(content,str): continue
        lines_by_name.setdefault(nm,set()).add(round(n["height"]/(fs*lh)))
    real=[]
    for h in hits:
        mm=re.search(r"'([^']+)'",h)
        if not mm or not wrapped.get(mm.group(1)): continue
        nm=mm.group(1)
        # cjkcheck 的每条报告都带 per=（它模拟的每行字数）。用它反推模拟行数，
        # 与实测行数比对：对不上说明这条抱怨描述的是一个并不存在的折行结果
        # ——比如实测只有 2 行，却报「L3 起于 '，'」。
        per=re.search(r"per=(\d+)", h)
        txt=re.search(r"全文='([^']*)'", h)
        if per:
            k=int(per.group(1))
            cand=[c for (n2,c,f2,l2) in m.values() if n2==nm and isinstance(c,str)]
            if txt: cand=[c for c in cand if c.startswith(txt.group(1)[:12])] or cand
            sim={-(-len(c.replace("\n",""))//k) for c in cand}
            if lines_by_name.get(nm) and not (sim & lines_by_name[nm]):
                continue
        Ln=re.search(r"\bL(\d+)\b", h)
        if Ln and lines_by_name.get(nm) and int(Ln.group(1)) > max(lines_by_name[nm]):
            continue
        real.append(h)
    if real:
        total_real+=1
        print(f"--- {p.stem} · 实测确有折行 {len(real)} 条（cjkcheck 原报 {len(hits)} 条）---")
        for h in dict.fromkeys(real): print("   ",h[:118])
print(f"\n实测确有折行的模板: {total_real} / {len(TARGETS)}")
