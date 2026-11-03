"""Move the Hyprland Wayland work in crates/linux under one `hyprland` module.

Driven by scripts/path/hyprland-module.rules. The run reads the linux crate's
module tree from lib.rs, moves the files the rules name, and rewrites every
`mod` declaration, `use` path, expression path, `#[path]` attribute and
visibility that the move touches. Only paths, module declarations and the
visibility needed to keep the same callers compiling change.

Usage: hyprland_module.py <repo-root> [--rules FILE] [--base REV] [--check]
"""
import argparse
import fnmatch
import os
import re
import shutil
import subprocess
import sys

CRATE_SRC = "crates/linux/src"
IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
MARKERS = ("self", "*")


class Fail(Exception):
    pass


def mask_source(src):
    """Comments and literals become spaces (newlines kept), so offsets match src."""
    out = list(src)
    lits = []
    i, n = 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
            continue
        if src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
            continue
        if c == "r" or c == "b" or c == "c":
            m = re.match(r'(?:br|cr|r)(#*)"', src[i:]) if (i == 0 or not re.match(r"[A-Za-z0-9_]", src[i - 1])) else None
            if m:
                close = '"' + m.group(1)
                body = i + m.end()
                j = src.find(close, body)
                j = n if j < 0 else j
                lits.append((i, j + len(close), src[body:j]))
                blank(i, j + len(close))
                i = j + len(close)
                continue
        if c == '"':
            start = i - 1 if i > 0 and src[i - 1] in "bc" and (i < 2 or not re.match(r"[A-Za-z0-9_]", src[i - 2])) else i
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            raw = src[i + 1:j]
            lits.append((start, j + 1, raw.replace('\\"', '"').replace("\\\\", "\\")))
            blank(start, j + 1)
            i = j + 1
            continue
        if c == "'":
            m = re.match(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'", src[i:i + 16])
            if m:
                blank(i, i + m.end())
                i += m.end()
                continue
        i += 1
    return "".join(out), lits


def balanced(code, i):
    pairs = {"(": ")", "[": "]", "{": "}"}
    stack = [pairs[code[i]]]
    j = i + 1
    while j < len(code) and stack:
        ch = code[j]
        if ch in pairs:
            stack.append(pairs[ch])
        elif ch == stack[-1]:
            stack.pop()
        j += 1
    return j


def line_start(text, pos):
    return text.rfind("\n", 0, pos) + 1


def item_start(src, mask, kw):
    """Start of the item at kw, with its outer attributes and docs but no inner docs before them."""
    pos = max(mask.rfind(";", 0, kw), mask.rfind("{", 0, kw), mask.rfind("}", 0, kw)) + 1
    while pos < kw:
        nl = src.find("\n", pos)
        if nl < 0 or nl >= kw:
            break
        line = src[pos:nl].strip()
        if line == "" or line.startswith("//!") or line.startswith("#!["):
            pos = nl + 1
            continue
        break
    first = line_start(src, pos)
    return first if src[first:pos].strip() == "" else pos


def line_depths(mask, region):
    """Brace depth at the start of each line inside region, relative to the region."""
    depth, out = 0, {region[0]: 0}
    for i in range(region[0], region[1]):
        ch = mask[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "\n":
            out[i + 1] = depth
    return out


VIS_TAIL = re.compile(r"(pub\s*(?:\(\s*(?:crate|super|self|in\s+[^)]*?)\s*\))?)\s*$")
VIS_ANY = re.compile(r"(?<![\w$])pub\s*\(\s*(crate|super|self|in\s+([^)]*?))\s*\)")


class FileInfo:
    def __init__(self, repo, rel, modpath, mod_rs_like):
        self.rel = rel
        self.modpath = modpath
        self.mod_rs_like = mod_rs_like
        with open(os.path.join(repo, rel), encoding="utf-8") as fh:
            self.src = fh.read()
        self.mask, self.lits = mask_source(self.src)
        self.inline = []
        for m in re.finditer(rf"(?<![\w$])mod\s+({IDENT})\s*\{{", self.mask):
            open_at = m.end() - 1
            self.inline.append((m.group(1), m.start(), open_at, balanced(self.mask, open_at)))
        self.decls = []
        self.uses = []

    def inline_stack(self, pos):
        return [name for name, _, open_at, close in sorted(self.inline, key=lambda t: t[1]) if open_at < pos < close]

    def ctx_at(self, pos):
        return tuple(self.modpath) + tuple(self.inline_stack(pos))

    def lit_in(self, a, b):
        return [lit for lit in self.lits if a <= lit[0] and lit[1] <= b]


class Decl:
    def __init__(self, info, m):
        self.file = info.rel
        self.name = m.group(1)
        self.kw = m.start()
        self.semi = m.end()
        self.start = item_start(info.src, info.mask, self.kw)
        end = info.src.find("\n", self.semi)
        self.end = len(info.src) if end < 0 else end + 1
        head = info.mask[self.start:self.kw]
        vm = VIS_TAIL.search(head)
        self.vis = re.sub(r"\s+", "", vm.group(1)) if vm else ""
        self.vis_span = (self.start + vm.start(1), self.start + vm.end(1)) if vm else None
        self.path_lit = None
        for a, b, value in info.lit_in(self.start, self.kw):
            if re.search(r"(?<![\w])path\s*=\s*$", info.mask[self.start:a]):
                self.path_lit = (a, b, value)
        self.stack = info.inline_stack(self.kw)
        self.parent = info.ctx_at(self.kw)


class Tree:
    def __init__(self, repo):
        self.repo = repo
        self.files = {}
        self.modules = {(): CRATE_SRC + "/lib.rs"}
        self.decl_of = {}
        self.file_of_mod = {}
        self.via_path = {}
        self.parse(CRATE_SRC + "/lib.rs", (), True)

    def parse(self, rel, modpath, mod_rs_like):
        if rel in self.files:
            raise Fail(f"{rel} is declared twice")
        info = FileInfo(self.repo, rel, modpath, mod_rs_like)
        self.files[rel] = info
        self.file_of_mod[modpath] = rel
        for name, _, open_at, _ in info.inline:
            self.modules[info.ctx_at(open_at) + (name,)] = rel
        here = os.path.dirname(rel)
        owned = here if mod_rs_like else os.path.join(here, os.path.basename(rel)[:-3])
        for m in re.finditer(rf"(?<![\w$])mod\s+({IDENT})\s*;", info.mask):
            d = Decl(info, m)
            info.decls.append(d)
            child = d.parent + (d.name,)
            if d.path_lit:
                base = os.path.join(owned, *d.stack) if d.stack else here
                target = os.path.normpath(os.path.join(base, d.path_lit[2]))
                child_mod_rs = True
            else:
                base = os.path.join(owned, *d.stack)
                flat = os.path.join(base, d.name + ".rs")
                nested = os.path.join(base, d.name, "mod.rs")
                target = flat if os.path.exists(os.path.join(self.repo, flat)) else nested
                child_mod_rs = target.endswith("mod.rs")
            if not os.path.exists(os.path.join(self.repo, target)):
                raise Fail(f"{rel} declares `mod {d.name};` but {target} does not exist")
            self.modules[child] = target
            self.decl_of[child] = d
            self.via_path[child] = bool(d.path_lit)
            self.parse(target, child, child_mod_rs)

    def subtree(self, modpath):
        return {m: f for m, f in self.modules.items() if m[:len(modpath)] == modpath}

    def children(self, modpath):
        return [m for m in self.decl_of if len(m) == len(modpath) + 1 and m[:-1] == modpath]


def mod_of_dir(tree, rel_dir):
    owner = os.path.join(rel_dir, "mod.rs")
    info = tree.files.get(owner)
    return info.modpath if info else None


def mod_path_of_dir(rel_dir):
    parts = os.path.relpath(rel_dir, CRATE_SRC).split(os.sep)
    return tuple(p for p in parts if p != ".")


class Plan:
    def __init__(self):
        self.roots = []
        self.children_rules = []
        self.newloc = {}
        self.dests = []
        self.relocated = []

    def map_ctx(self, path):
        path = tuple(path)
        for src_mod, dst_mod, _ in self.children_rules:
            if path == src_mod:
                return dst_mod
        return self.map_path(path, allow_children_root=True)

    def map_path(self, path, allow_children_root=False):
        path = tuple(path)
        best = None
        for old, new in self.roots:
            if path[:len(old)] == old and (best is None or len(old) > len(best[0])):
                best = (old, new)
        if best:
            return best[1] + path[len(best[0]):]
        for src_mod, dst_mod, excluded in self.children_rules:
            if path[:len(src_mod)] != src_mod:
                continue
            if len(path) == len(src_mod):
                if allow_children_root:
                    return path
                raise Fail(f"a path names the module crate::{'::'.join(src_mod)} itself, whose contents move; "
                           "import its items by full path instead")
            if path[len(src_mod)] in MARKERS:
                raise Fail(f"a `use` imports crate::{'::'.join(src_mod)} with `self` or `*`, whose contents move")
            if path[len(src_mod)] == excluded:
                return path
            return dst_mod + path[len(src_mod):]
        return path

    def moved(self, path):
        return self.map_path(path, allow_children_root=True) != tuple(path)


def read_rules(path):
    rules, into = [], None
    with open(path, encoding="utf-8") as fh:
        for number, line in enumerate(fh, 1):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 2 or parts[0] not in ("into", "children", "module", "keep"):
                raise Fail(f"{path}:{number}: expected `into|children|module|keep <path>`")
            kind, arg = parts
            arg = os.path.normpath(arg)
            if kind == "into":
                into = arg
                continue
            if kind != "keep" and into is None:
                raise Fail(f"{path}:{number}: `{kind}` comes before any `into`")
            rules.append((kind, arg, into))
    return rules


def build_plan(tree, rules):
    plan = Plan()
    claimed = {}
    for kind, arg, into in rules:
        if kind == "keep":
            continue
        if not into.startswith(CRATE_SRC + os.sep):
            raise Fail(f"`into {into}` is outside {CRATE_SRC}")
        if into not in plan.dests:
            plan.dests.append(into)
        dst_mod = mod_path_of_dir(into)
        if kind == "children":
            src_mod = mod_of_dir(tree, arg)
            if src_mod is None:
                continue
            excluded = dst_mod[-1] if dst_mod[:-1] == src_mod else None
            info = tree.files[os.path.join(arg, "mod.rs")]
            movers = [c for c in tree.children(src_mod) if c[-1] != excluded]
            others = [d for d in info.decls if d.name != excluded]
            leftover = info.mask
            for d in info.decls:
                if d.name == excluded:
                    leftover = leftover[:d.start] + " " * (d.end - d.start) + leftover[d.end:]
            if not movers and not others and leftover.strip() == "":
                continue
            plan.children_rules.append((src_mod, dst_mod, excluded))
            for m, f in tree.subtree(src_mod).items():
                if excluded and m[:len(src_mod) + 1] == src_mod + (excluded,):
                    continue
                if f in plan.newloc:
                    continue
                if tree.files.get(f) is None or tree.files[f].modpath != m:
                    continue
                rel = os.path.relpath(f, arg)
                if rel.startswith(".."):
                    raise Fail(f"{f} belongs to {arg} but sits outside it; place it by hand first")
                claim(plan, claimed, f, os.path.join(into, rel))
        elif kind == "module":
            matches = sorted(f for f in tree.files if fnmatch.fnmatch(f, arg))
            for f in matches:
                root = tree.files[f].modpath
                if tree.via_path.get(root):
                    raise Fail(f"{f} is declared through #[path]; move its declarer instead")
                parent_file = tree.decl_of[root].file
                if parent_file in plan.newloc:
                    raise Fail(f"{f} is named by `module` but its parent {parent_file} already moves")
                if tree.decl_of[root].stack:
                    raise Fail(f"{f} is declared inside an inline module in {parent_file}")
                plan.roots.append((root, dst_mod + (root[-1],)))
                base = os.path.dirname(f)
                for m, g in tree.subtree(root).items():
                    if tree.files.get(g) is None or tree.files[g].modpath != m:
                        continue
                    rel = os.path.relpath(g, base)
                    if rel.startswith(".."):
                        raise Fail(f"{g} belongs to {f} but sits outside {base}; place it by hand first")
                    claim(plan, claimed, g, os.path.join(into, rel))
                plan.relocated.append((root, into))
    return plan


def claim(plan, claimed, src, dst):
    if dst in claimed and claimed[dst] != src:
        raise Fail(f"{src} and {claimed[dst]} would both land at {dst}")
    claimed[dst] = src
    plan.newloc[src] = dst


def added_files(repo, base):
    out = subprocess.run(["git", "-C", repo, "diff", "--no-renames", "--diff-filter=A", "--name-only",
                          base, "HEAD", "--", CRATE_SRC], capture_output=True, text=True)
    if out.returncode != 0:
        raise Fail("git diff against the base failed:\n" + out.stderr)
    return [f for f in out.stdout.split() if f.endswith(".rs")]


def check_unknown(repo, tree, plan, rules, base):
    keeps = [arg for kind, arg, _ in rules if kind == "keep"]
    dests = [into for kind, _, into in rules if kind != "keep" and into]
    bridges = set()
    for into in dests:
        d = os.path.dirname(into)
        while d.startswith(CRATE_SRC + os.sep):
            bridges.add(os.path.join(d, "mod.rs"))
            d = os.path.dirname(d)
    unknown = []
    for f in added_files(repo, base):
        if f in plan.newloc or f in bridges:
            continue
        if any(f.startswith(into + os.sep) for into in dests):
            continue
        if any(fnmatch.fnmatch(f, k) for k in keeps):
            continue
        unknown.append(f)
    if unknown:
        raise Fail("these files are added against the base but no rule places them; add a `module`, "
                   "`children` or `keep` rule to the rules file:\n  " + "\n  ".join(unknown))
    stray = [f for f in plan.newloc if f not in tree.files]
    if stray:
        raise Fail("no module declares " + ", ".join(stray))


def resolve(segs, ctx, modules):
    if not segs:
        return None
    s0 = segs[0]
    if s0 == "crate":
        return tuple(segs[1:])
    if s0 in ("super", "self"):
        base = list(ctx)
        i = 1 if s0 == "self" else 0
        while i < len(segs) and segs[i] == "super":
            if not base:
                return None
            base.pop()
            i += 1
        return tuple(base) + tuple(segs[i:])
    if tuple(ctx) + (s0,) in modules:
        return tuple(ctx) + tuple(segs)
    return None


def scope_of(vis, container):
    if vis in ("pub", "pub(crate)"):
        return ()
    if vis == "pub(super)":
        return tuple(container[:-1])
    if vis in ("", "pub(self)"):
        return tuple(container)
    m = re.fullmatch(r"pub\(in(.*)\)", vis)
    if m:
        segs = [s for s in m.group(1).replace(" ", "").split("::") if s]
        return resolve(segs, container, {}) or ()
    return ()


def covers(scope, ctx):
    return tuple(ctx[:len(scope)]) == tuple(scope)


TOKEN = re.compile(rf"::|[{{}},*;]|{IDENT}|\S")


def parse_use(text, offset):
    toks = [(m.group(0), m.start() + offset, m.end() + offset) for m in TOKEN.finditer(text)]
    pos = [0]

    def peek():
        return toks[pos[0]][0] if pos[0] < len(toks) else None

    def take(expect=None):
        tok = toks[pos[0]]
        if expect and tok[0] != expect:
            raise ValueError(f"expected {expect}, found {tok[0]}")
        pos[0] += 1
        return tok

    def tree(prefix):
        leaves, segs = [], list(prefix)
        if peek() == "*":
            take()
            return [(segs + ["*"], None)]
        if peek() == "{":
            return group(segs)
        while True:
            tok = take()
            if not re.fullmatch(IDENT, tok[0]):
                raise ValueError(f"unexpected {tok[0]}")
            segs.append(tok[0])
            if peek() == "::":
                take()
                if peek() == "*":
                    take()
                    return [(segs + ["*"], None)]
                if peek() == "{":
                    return group(segs)
                continue
            alias = None
            if peek() == "as":
                take()
                alias = take()[0]
            leaves.append((segs, alias))
            return leaves

    def group(segs):
        take("{")
        leaves = []
        while peek() != "}":
            leaves += tree(segs)
            if peek() == ",":
                take()
        take("}")
        return leaves

    if peek() == "::":
        return None, None, None
    prefix_toks = []
    p = 0
    while p < len(toks) and re.fullmatch(IDENT, toks[p][0]) and toks[p][0] != "as":
        prefix_toks.append(toks[p])
        if p + 1 < len(toks) and toks[p + 1][0] == "::":
            p += 2
        else:
            break
    leaves = tree([])
    if pos[0] != len(toks):
        raise ValueError("trailing tokens")
    span = (prefix_toks[0][1], prefix_toks[-1][2]) if prefix_toks else None
    return leaves, [t[0] for t in prefix_toks], span


class Rewriter:
    def __init__(self, repo, tree, plan):
        self.repo = repo
        self.tree = tree
        self.plan = plan
        self.edits = {}
        self.reach = []
        self.new_modules = {plan.map_ctx(m) for m in tree.modules}
        for into in plan.dests:
            d = mod_path_of_dir(into)
            while d:
                self.new_modules.add(d)
                d = d[:-1]

    def edit(self, rel, a, b, text):
        self.edits.setdefault(rel, []).append((a, b, text))

    def file_moved(self, rel):
        return rel in self.plan.newloc

    def emit(self, segs, ctx_old, ctx_new):
        abs_old = resolve(segs, ctx_old, self.tree.modules)
        if abs_old is None:
            return None, None
        abs_new = self.plan.map_path(abs_old)
        if segs[0] != "crate" and resolve(segs, ctx_new, self.new_modules) == abs_new:
            return list(segs), abs_old
        return ["crate"] + list(abs_new), abs_old

    def note_reach(self, info, abs_old, ctx_new):
        if self.file_moved(info.rel) and not self.plan.moved(abs_old):
            self.reach.append((abs_old, ctx_new))

    def run(self):
        for rel, info in self.tree.files.items():
            self.rewrite_file(info)
        self.widen()
        self.fix_path_literals()

    def rewrite_file(self, info):
        moved = self.file_moved(info.rel)
        taken = []
        rebuilt = []
        for m in re.finditer(r"(?<![\w$])use\s", info.mask):
            semi = info.mask.find(";", m.end())
            if semi < 0:
                continue
            body = info.mask[m.end():semi]
            if "$" in body:
                continue
            try:
                leaves, prefix, prefix_span = parse_use(body, m.end())
            except (ValueError, IndexError):
                if re.search(r"(?<![\w$])(crate|super|self)\s*::", body):
                    raise Fail(f"{info.rel}: cannot parse `use{body};`")
                continue
            taken.append((m.start(), semi + 1))
            if leaves is None:
                continue
            ctx_old = info.ctx_at(m.start())
            ctx_new = self.plan.map_ctx(ctx_old)
            emitted = []
            changed = False
            for segs, alias in leaves:
                new, abs_old = self.emit(segs, ctx_old, ctx_new)
                if new is None:
                    emitted.append((list(segs), alias))
                    continue
                self.note_reach(info, abs_old, ctx_new)
                emitted.append((new, alias))
                changed |= new != list(segs)
            if not changed:
                continue
            k = len(prefix)
            heads = {tuple(new[:len(new) - (len(segs) - k)]) for (segs, _), (new, _) in zip(leaves, emitted)
                     if len(segs) >= k}
            tails_ok = all(new[len(new) - (len(segs) - k):] == segs[k:] for (segs, _), (new, _) in zip(leaves, emitted))
            if prefix_span and len(heads) == 1 and tails_ok and all(len(s) > k or len(leaves) == 1 for s, _ in leaves):
                head = list(heads.pop())
                self.edit(info.rel, prefix_span[0], prefix_span[1], "::".join(head))
                continue
            start = item_start(info.src, info.mask, m.start())
            pre = info.mask[start:m.start()]
            vm = VIS_TAIL.search(pre)
            vis_text = vm.group(1) if vm else ""
            if vm:
                vis_text = self.new_vis_text(info, start + vm.start(1), vis_text) or vis_text
            attrs = info.src[start:start + vm.start(1)] if vm else info.src[start:m.start()]
            indent = re.match(r"[ \t]*", info.src[start:]).group(0)
            attrs = attrs.strip()
            groups = {}
            for new, alias in emitted:
                groups.setdefault(tuple(new[:-1]), []).append(new[-1] + (f" as {alias}" if alias else ""))
            stmts = []
            for container, members in groups.items():
                if members == ["self"]:
                    path = "::".join(container)
                elif len(members) == 1:
                    path = "::".join(container + (members[0],))
                else:
                    path = "::".join(container) + "::{" + ", ".join(members) + "}"
                line = f"{indent}{vis_text + ' ' if vis_text else ''}use {path};"
                stmts.append(f"{indent}{attrs}\n{line}" if attrs else line)
            self.edit(info.rel, start, semi + 1, "\n".join(stmts))
            rebuilt.append((start, semi + 1))

        def inside(pos, spans):
            return any(a <= pos < b for a, b in spans)

        vis_spans = []
        for m in VIS_ANY.finditer(info.mask):
            vis_spans.append((m.start(), m.end()))
            if not moved or inside(m.start(), rebuilt):
                continue
            new = self.new_vis_text(info, m.start(), re.sub(r"\s+", " ", m.group(0)))
            if new is not None:
                self.edit(info.rel, m.start(), m.end(), new)

        for m in re.finditer(rf"(?<![\w$:])(?:crate|super|self)(?:\s*::\s*{IDENT})+", info.mask):
            if inside(m.start(), taken) or inside(m.start(), vis_spans):
                continue
            segs = [s.strip() for s in m.group(0).split("::")]
            ctx_old = info.ctx_at(m.start())
            ctx_new = self.plan.map_ctx(ctx_old)
            new, abs_old = self.emit(segs, ctx_old, ctx_new)
            if new is None:
                continue
            self.note_reach(info, abs_old, ctx_new)
            if new != segs:
                self.edit(info.rel, m.start(), m.end(), "::".join(new))

        bound = set()
        for a, b in taken:
            try:
                leaves, _, _ = parse_use(info.mask[a + 3:b - 1], 0)
            except (ValueError, IndexError):
                continue
            for segs, alias in leaves or []:
                bound.add(alias or segs[-1])
        bare = rf"(?<![\w$:.])({IDENT})((?:\s*::\s*{IDENT})+)"
        for m in re.finditer(bare, info.mask):
            first = m.group(1)
            if first in ("crate", "super", "self", "Self") or first in bound:
                continue
            if inside(m.start(), taken) or inside(m.start(), vis_spans):
                continue
            ctx_old = info.ctx_at(m.start())
            if ctx_old + (first,) not in self.tree.modules:
                continue
            segs = [s.strip() for s in m.group(0).split("::")]
            ctx_new = self.plan.map_ctx(ctx_old)
            new, abs_old = self.emit(segs, ctx_old, ctx_new)
            if new is None:
                continue
            self.note_reach(info, abs_old, ctx_new)
            if new != segs:
                self.edit(info.rel, m.start(), m.end(), "::".join(new))

    def new_vis_text(self, info, pos, text):
        """The visibility an item in a moved file needs so its old callers still reach it, or None to keep text."""
        if not self.file_moved(info.rel):
            return None
        vis = re.sub(r"\s+", "", text).replace("pub(in", "pub(in ")
        ctx_old = info.ctx_at(pos)
        ctx_new = self.plan.map_ctx(ctx_old)
        if vis in ("pub", "pub(crate)"):
            return None
        target = self.plan.map_ctx(scope_of(vis, ctx_old))
        if vis == "pub(super)" and tuple(ctx_new[:-1]) == target:
            return None
        if vis == "pub(self)" and tuple(ctx_new) == target:
            return None
        if vis.startswith("pub(in ") and target == scope_of(vis, ctx_new):
            return None
        return "pub(crate)"

    def widen(self):
        done = set()
        for abs_old, ctx_new in self.reach:
            for i in range(1, len(abs_old) + 1):
                prefix = tuple(abs_old[:i])
                if prefix in self.tree.decl_of:
                    d = self.tree.decl_of[prefix]
                    if not covers(scope_of(d.vis, d.parent), ctx_new) and (d.file, d.kw) not in done:
                        done.add((d.file, d.kw))
                        self.set_vis(d.file, d.vis_span, d.kw, d.name)
                    continue
                name = abs_old[i - 1]
                if name in MARKERS:
                    break
                if prefix in self.tree.modules:
                    continue
                self.widen_item(tuple(abs_old[:i - 1]), name, ctx_new, done)
                break

    def set_vis(self, rel, span, kw, name):
        if span:
            self.edit(rel, span[0], span[1], "pub(crate)")
        else:
            self.edit(rel, kw, kw, "pub(crate) ")
        print(f"widened `{name}` in {rel} to pub(crate) so the moved code still reaches it")

    def widen_item(self, container, name, ctx_new, done):
        rel = self.tree.modules.get(container)
        if rel is None:
            return
        info = self.tree.files[rel]
        region = (0, len(info.mask))
        if info.modpath != container:
            for iname, _, open_at, close in info.inline:
                if info.ctx_at(open_at) + (iname,) == container:
                    region = (open_at + 1, close - 1)
        depth = line_depths(info.mask, region)
        item = re.compile(rf"(?m)^[ \t]*(pub(?:\s*\([^)]*\))?\s+)?(?:(?:default|const|async|unsafe|extern)\s+)*"
                          rf"(?:fn|struct|enum|trait|type|const|static(?:\s+mut)?|union|mod)\s+{re.escape(name)}\b")
        for m in item.finditer(info.mask, region[0], region[1]):
            if depth.get(max(region[0], m.start()), 1) != 0:
                continue
            vis = re.sub(r"\s+", "", m.group(1) or "")
            if covers(scope_of(vis, container), ctx_new) or (rel, m.start()) in done:
                return
            done.add((rel, m.start()))
            kw = m.start() + len(m.group(0)) - len(m.group(0).lstrip())
            span = (m.start(1), m.start(1) + len(m.group(1).rstrip())) if m.group(1) else None
            self.set_vis(rel, span, kw, name)
            return
        for m in re.finditer(r"(?<![\w$])use\s", info.mask[:region[1]]):
            if m.start() < region[0] or depth.get(max(region[0], line_start(info.mask, m.start())), 1) != 0:
                continue
            semi = info.mask.find(";", m.end())
            try:
                leaves, _, _ = parse_use(info.mask[m.end():semi], 0)
            except (ValueError, IndexError):
                continue
            if not any((alias or segs[-1]) == name for segs, alias in leaves or []):
                continue
            pre = info.mask[item_start(info.src, info.mask, m.start()):m.start()]
            vm = VIS_TAIL.search(pre)
            vis = re.sub(r"\s+", "", vm.group(1)) if vm else ""
            if covers(scope_of(vis, container), ctx_new) or (rel, m.start()) in done:
                return
            done.add((rel, m.start()))
            base = m.start() - len(pre)
            span = (base + vm.start(1), base + vm.end(1)) if vm else None
            self.set_vis(rel, span, m.start(), name)
            return
        print(f"warning: could not find `{name}` in {rel} to check that the moved code still reaches it",
              file=sys.stderr)

    def fix_path_literals(self):
        for child, is_path in self.tree.via_path.items():
            if not is_path:
                continue
            d = self.tree.decl_of[child]
            target = self.tree.modules[child]
            new_target = self.plan.newloc.get(target, target)
            declarer = d.file
            new_declarer = self.plan.newloc.get(declarer, declarer)
            if new_target == target and new_declarer == declarer:
                continue
            info = self.tree.files[declarer]
            here = os.path.dirname(new_declarer)
            owned = here if info.mod_rs_like else os.path.join(here, os.path.basename(new_declarer)[:-3])
            base = os.path.join(owned, *d.stack) if d.stack else here
            lit = os.path.relpath(new_target, base).replace(os.sep, "/")
            if lit != d.path_lit[2]:
                self.edit(declarer, d.path_lit[0], d.path_lit[1], f'"{lit}"')
        for rel, info in self.tree.files.items():
            if rel not in self.plan.newloc:
                continue
            for m in re.finditer(r"\binclude(?:_str|_bytes)?\s*!\s*[\(\[\{]", info.mask):
                for a, b, value in info.lit_in(m.end(), m.end() + 400)[:1]:
                    target = os.path.normpath(os.path.join(os.path.dirname(rel), value))
                    new_target = self.plan.newloc.get(target, target)
                    lit = os.path.relpath(new_target, os.path.dirname(self.plan.newloc[rel])).replace(os.sep, "/")
                    if lit != value:
                        self.edit(rel, a, b, f'"{lit}"')


def apply_edits(src, edits):
    edits = sorted(set(edits), key=lambda e: (e[0], e[1]))
    out, last = [], 0
    for a, b, text in edits:
        if a < last:
            raise Fail("two rewrites overlap; this file needs a hand edit")
        out.append(src[last:a])
        out.append(text)
        last = b
    out.append(src[last:])
    return "".join(out)


def relocated_decl(tree, plan, root):
    d = tree.decl_of[root]
    info = tree.files[d.file]
    text = info.src[d.start:d.end]
    old_vis = d.vis
    target = plan.map_ctx(scope_of(old_vis, d.parent))
    new_parent = plan.map_path(root)[:-1]
    if old_vis in ("pub", "pub(crate)"):
        vis = old_vis
    elif old_vis == "" and target == new_parent:
        vis = ""
    elif old_vis == "pub(super)" and target == new_parent[:-1]:
        vis = old_vis
    else:
        vis = "pub(crate)"
    kw_rel = d.kw - d.start
    if d.vis_span:
        head = text[:d.vis_span[0] - d.start]
    else:
        head = text[:kw_rel]
    head = "\n".join(line.strip() for line in head.splitlines()).strip()
    line = f"{vis + ' ' if vis else ''}mod {d.name};"
    return f"{head}\n{line}\n" if head else f"{line}\n"


def insert_decls(text, decls):
    """Add `mod` declarations after the file's last plain one, and attributed ones after its last one."""
    if not decls:
        return text
    if text and not text.endswith("\n"):
        text += "\n"
    mask, _ = mask_source(text)
    depths = line_depths(mask, (0, len(mask)))
    plain_end = any_end = None
    for m in re.finditer(rf"(?<![\w$])mod\s+{IDENT}\s*;", mask):
        if depths.get(line_start(mask, m.start()), 0) != 0:
            continue
        nl = text.find("\n", m.end())
        end = len(text) if nl < 0 else nl + 1
        any_end = end
        if "#[" not in mask[item_start(text, mask, m.start()):m.start()]:
            plain_end = end
    plain = "".join(d for d in decls if "#[" not in d)
    attributed = "".join(d for d in decls if "#[" in d)
    if any_end is None:
        parts = [p for p in (text.rstrip("\n"), plain.rstrip("\n"), attributed.rstrip("\n")) if p]
        return "\n\n".join(parts) + "\n"
    if plain_end is None:
        plain_end = any_end
    out, last = [], 0
    for pos, chunk in sorted([(plain_end, plain), (any_end, attributed)], key=lambda p: p[0]):
        out += [text[last:pos], chunk]
        last = pos
    out.append(text[last:])
    return "".join(out)


def declare_in(texts, owner, name):
    text = texts.get(owner, "")
    mask, _ = mask_source(text)
    if re.search(rf"(?<![\w$])mod\s+{re.escape(name)}\s*[;{{]", mask):
        return False
    texts[owner] = insert_decls(text, [f"pub(crate) mod {name};\n"])
    return True


def execute(repo, rules_path, base, check):
    rules = read_rules(rules_path)
    tree = Tree(repo)
    plan = build_plan(tree, rules)
    check_unknown(repo, tree, plan, rules, base)
    if not plan.newloc:
        print("hyprland-module: nothing to move; the tree already matches the rules")
        return 0
    if check:
        for src, dst in sorted(plan.newloc.items()):
            print(f"would move {src} -> {dst}")
        return 1
    rw = Rewriter(repo, tree, plan)
    rw.run()
    for root, _ in plan.roots:
        d = tree.decl_of[root]
        rw.edit(d.file, d.start, d.end, "")
    for src, dst in sorted(plan.newloc.items()):
        if dst not in plan.newloc and os.path.exists(os.path.join(repo, dst)):
            raise Fail(f"{dst} already exists; moving {src} there would overwrite it")
    texts, removed = {}, set()
    for rel, info in tree.files.items():
        if rel not in rw.edits and rel not in plan.newloc:
            continue
        new_text = apply_edits(info.src, rw.edits.get(rel, []))
        dst = plan.newloc.get(rel, rel)
        if dst != rel:
            removed.add(rel)
        texts[dst] = new_text
    on_disk = lambda rel: rel not in removed and os.path.exists(os.path.join(repo, rel))
    for root, into in plan.relocated:
        owner = os.path.join(into, "mod.rs")
        if owner not in texts:
            texts[owner] = open(os.path.join(repo, owner), encoding="utf-8").read() if on_disk(owner) else ""
        texts[owner] = insert_decls(texts[owner], [relocated_decl(tree, plan, root)])
    for into in plan.dests:
        owner = os.path.join(into, "mod.rs")
        if owner not in texts and not on_disk(owner):
            texts[owner] = ""
        d = into
        while d != CRATE_SRC:
            parent = os.path.dirname(d)
            owner = None
            for candidate in (os.path.join(parent, "mod.rs"), parent + ".rs"):
                if candidate in texts or on_disk(candidate):
                    owner = candidate
                    break
            if parent == CRATE_SRC:
                owner = os.path.join(CRATE_SRC, "lib.rs")
            if owner is None:
                owner = os.path.join(parent, "mod.rs")
                texts[owner] = ""
            if owner not in texts:
                texts[owner] = open(os.path.join(repo, owner), encoding="utf-8").read()
            if not declare_in(texts, owner, os.path.basename(d)):
                break
            d = parent
    for rel in sorted(removed):
        path = os.path.join(repo, rel)
        if os.path.exists(path):
            os.remove(path)
    for rel in list(texts):
        if rel not in removed and rel not in plan.newloc.values() and on_disk(rel):
            with open(os.path.join(repo, rel), encoding="utf-8") as fh:
                if fh.read() == texts[rel]:
                    del texts[rel]
    for rel, text in sorted(texts.items()):
        path = os.path.join(repo, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text if text.endswith("\n") or not text else text + "\n")
    for rel in sorted(removed, key=len, reverse=True):
        d = os.path.dirname(os.path.join(repo, rel))
        while d.startswith(os.path.join(repo, CRATE_SRC)) and os.path.isdir(d) and not os.listdir(d):
            os.rmdir(d)
            d = os.path.dirname(d)
    rustfmt = shutil.which("rustfmt")
    if rustfmt is None:
        raise Fail("rustfmt is not on PATH; the moved files are written but not formatted")
    fmt = subprocess.run([rustfmt, "--edition", "2024"] + sorted(os.path.join(repo, r) for r in texts),
                         capture_output=True, text=True)
    if fmt.returncode != 0:
        raise Fail("rustfmt failed on the rewritten files:\n" + fmt.stderr)
    for src, dst in sorted(plan.newloc.items()):
        print(f"moved {src} -> {dst}")
    for rel in sorted(set(texts) - set(plan.newloc.values())):
        print(f"rewrote {rel}")
    return 0


def default_base(repo):
    for ref in ("origin/main", "main", "upstream/main"):
        out = subprocess.run(["git", "-C", repo, "merge-base", "HEAD", ref], capture_output=True, text=True)
        if out.returncode == 0:
            return out.stdout.strip()
    raise Fail("no main branch to diff against; pass --base")


def main():
    parser = argparse.ArgumentParser(prog="hyprland-module.sh", description=__doc__.split("\n\n")[0])
    parser.add_argument("repo")
    parser.add_argument("--rules", default=None)
    parser.add_argument("--base", default=None)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    repo = os.path.realpath(args.repo)
    rules = args.rules or os.path.join(repo, "scripts", "path", "hyprland-module.rules")
    os.chdir(repo)
    try:
        dirty = subprocess.run(["git", "status", "--porcelain", "--", CRATE_SRC], capture_output=True, text=True)
        if dirty.stdout.strip() and not args.check:
            raise Fail(f"{CRATE_SRC} has uncommitted changes; commit or discard them first")
        base = args.base or default_base(repo)
        return execute(repo, rules, base, args.check)
    except Fail as error:
        print(f"hyprland-module: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
