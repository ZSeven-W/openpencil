#!/usr/bin/env bash
# tools/check-widget-boundary.sh — Step 1b §1.4 widget boundary invariant.
#
# Per spec §1.4: widget logic (Widget impls, layout/paint/access_node)
# lives in `crates/openpencil-shell-core/src/widgets/`. shell-web is
# allowed exactly one glue file (`crates/openpencil-shell-web/src/
# widget_host.rs`); the `// glue:` marker on its paint signature is
# the only line allowed to host a `fn paint(` that drives widgets.
# Any function or file in shell-web that pulls
# `openpencil_shell_core::widgets::*` must live in widget_host.rs
# (the spec's "any function pulling shell-core widgets" clause).
#
# Reverse direction: shell-core/src/widgets/ MUST contain four impl
# files (tree, prop_row, dropdown, text_input) AND each file must
# carry a real `impl Widget for X` — a stale file with the impl
# removed must not silently pass.
#
# Exit semantics:
#   0   PASS — both invariants hold.
#   1   FAIL — widget logic leaked into shell-web OR shell-core
#       widget impl files are missing / empty.

set -euo pipefail

WEB_SRC="crates/openpencil-shell-web/src"
CORE_WIDGETS="crates/openpencil-shell-core/src/widgets"

fail_lines=()

# ---------------------------------------------------------------------
# Forward F1: no `impl Widget for X` (with optional generic params and
# arbitrary namespace segments) anywhere under shell-web/src/.
# Even widget_host.rs is not allowed to host a Widget impl — all real
# impls live in shell-core. The `// glue:` exemption is for the paint
# signature only, not for Widget trait impls (codex B4 R2 CONCERN-2).
# ---------------------------------------------------------------------
# `impl(<...>)?[[:space:]]+(ns::)*Widget[[:space:]]+for[[:space:]]`:
# - `(<[^>]+>)?` allows zero-or-one generic param block (`impl<T>`)
#   before the trait path (codex B4 R2 CONCERN-1).
# - `([[:alnum:]_]+::)*` allows zero or more namespace segments
#   before `Widget` (codex B4 R1 BLOCK fix).
# - The `[[:space:]]+for[[:space:]]` tail rules out structs named
#   WidgetHost / WidgetId.
impl_hits="$(grep -RInE \
    'impl(<[^>]+>)?[[:space:]]+([[:alnum:]_]+::)*Widget[[:space:]]+for[[:space:]]' \
    "${WEB_SRC}" 2>/dev/null || true)"
if [ -n "${impl_hits}" ]; then
  fail_lines+=("Forward F1: Widget trait impl in shell-web (must live in shell-core):" "${impl_hits}")
fi

# ---------------------------------------------------------------------
# Forward F2: no `fn layout(` / `fn access_node(` anywhere under
# shell-web/src/. These are widget-trait method signatures and have
# no business in the platform crate. (Note: `fn paint(` is allowed
# only on the `// glue:` marked line in widget_host.rs — handled by
# F3 below to keep the exemption tight.)
#
# Introduced alongside F1/F3/F4 in codex B4 R2 (the original single-
# regex check was split into per-direction blocks with named
# findings); kept stable through R3 so no in-line "fixed by …"
# citation was needed. Listed here for parity with F1/F3/F4/R1.
# ---------------------------------------------------------------------
trait_method_hits="$(grep -RInE \
    'fn[[:space:]]+(layout|access_node)\(' \
    "${WEB_SRC}" 2>/dev/null || true)"
if [ -n "${trait_method_hits}" ]; then
  fail_lines+=("Forward F2: Widget trait method (layout / access_node) in shell-web:" "${trait_method_hits}")
fi

# ---------------------------------------------------------------------
# Forward F3: `fn paint(` under shell-web is allowed ONLY when the
# line ALSO carries the `// glue:` marker AND the file is
# widget_host.rs. Tight exemption (codex B4 R2 CONCERN-2 — broad
# `// glue:` exemption could hide leaked impl/method lines if anyone
# tagged them; the F1/F2 checks above already scan everything, so
# F3's exemption only needs to bless the documented paint signature).
# ---------------------------------------------------------------------
paint_hits="$(grep -RInE \
    'fn[[:space:]]+paint\(' \
    "${WEB_SRC}" 2>/dev/null || true)"
if [ -n "${paint_hits}" ]; then
  illegal_paint="$(printf '%s\n' "${paint_hits}" \
    | grep -v '^crates/openpencil-shell-web/src/widget_host\.rs:.*// glue:' \
    || true)"
  if [ -n "${illegal_paint}" ]; then
    fail_lines+=("Forward F3: fn paint( outside widget_host.rs's // glue: line:" "${illegal_paint}")
  fi
fi

# ---------------------------------------------------------------------
# Forward F4: only widget_host.rs may import `openpencil_shell_core::
# widgets` in any form. Catches:
#   - `use openpencil_shell_core::widgets;`                 (direct)
#   - `use openpencil_shell_core::widgets::Foo;`            (direct)
#   - `use openpencil_shell_core::{widgets};`               (grouped)
#   - `use openpencil_shell_core::{widgets::Foo};`          (grouped)
#   - `openpencil_shell_core::widgets::call()`              (path expr)
#   - Multi-line grouped imports are NOT caught (single-line
#     scope only — code review enforces single-line use stmts in this
#     crate; the cost-benefit doesn't justify a multi-line parser).
#
# Codex B4 R2 BLOCK-fix introduced F4; R3 BLOCK refined for grouped
# import forms — the previous regex required a literal `::widgets`
# right after `openpencil_shell_core`, missing the brace form.
# Solution: any line mentioning `openpencil_shell_core` AND `widgets`
# is a hit (single grep pass picks both patterns up).
# ---------------------------------------------------------------------
core_ref_hits="$(grep -RIn 'openpencil_shell_core' "${WEB_SRC}" 2>/dev/null || true)"
if [ -n "${core_ref_hits}" ]; then
  # Filter to lines that ALSO mention `widgets`, then drop the
  # widget_host.rs allowance.
  illegal_imports="$(printf '%s\n' "${core_ref_hits}" \
    | grep 'widgets' \
    | grep -v '^crates/openpencil-shell-web/src/widget_host\.rs:' \
    || true)"
  if [ -n "${illegal_imports}" ]; then
    fail_lines+=("Forward F4: openpencil_shell_core::widgets reference outside widget_host.rs (covers direct + grouped use forms):" "${illegal_imports}")
  fi
fi

# ---------------------------------------------------------------------
# Reverse R1: each of the four expected widget impl files exists AND
# carries a real `impl Widget for X` line that is NOT a Rust
# line-comment.
#
# Codex B4 R2 CONCERN-3 introduced R1; R3 CONCERN refined the impl
# grep so a commented-out `// impl Widget for TextInput` stub does
# not silently satisfy the check. The stripping uses sed to drop
# `//`-prefixed content (after optional leading whitespace) before
# grepping; block comments (`/* … */`) are not handled because the
# Rust style in shell-core/src/widgets/ uses line comments only.
# ---------------------------------------------------------------------
required_widgets=("tree" "prop_row" "dropdown" "text_input")
for w in "${required_widgets[@]}"; do
  file="${CORE_WIDGETS}/${w}.rs"
  if [ ! -f "${file}" ]; then
    fail_lines+=("Reverse R1: ${file} missing")
    continue
  fi
  uncommented="$(sed -E 's@[[:space:]]*//.*@@' "${file}")"
  if ! printf '%s\n' "${uncommented}" \
       | grep -qE 'impl(<[^>]+>)?[[:space:]]+([[:alnum:]_]+::)*Widget[[:space:]]+for[[:space:]]'; then
    fail_lines+=("Reverse R1: ${file} has no live \`impl Widget for X\` (file exists but stale, or impl is commented out)")
  fi
done

# ---------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------
if [ "${#fail_lines[@]}" -gt 0 ]; then
  printf 'FAIL: widget boundary check\n' >&2
  printf '\n' >&2
  for line in "${fail_lines[@]}"; do
    printf '%s\n' "${line}" >&2
  done
  exit 1
fi

echo "PASS: widget boundary check"
