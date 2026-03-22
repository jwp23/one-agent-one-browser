# Next Session Prompt

Continue work in `/home/mordant23/workspace/jwp23/one-agent-one-browser/.worktrees/in-tree-js-engine`
on branch `in-tree-js-engine`.

Read `AGENTS.md` and continue the no-third-party in-tree JS engine/runtime work.

Current checkpoint after commit `1b06697` (March 21, 2026):
- Commit: `js: preserve globals across script sources`
- Since `03173bf`, the JS engine/runtime gained:
  - trace-only call-chain diagnostics for `OAB_TRACE_JS_ERRORS=1`
  - `Array.prototype.join`
  - `Array.prototype.push` as a callable prototype method, including `push.apply(...)`
  - `encodeURIComponent`
  - detached `<script>` property assignment for `src` / `onload` / `onerror`
  - array own-properties support, so arrays can override methods like `RLQ.push`
  - persistent JS runtime/global state across multiple script sources
- `cargo test --lib` passes with 136 tests and no warnings
- Do not remove the Wikipedia appearance fallback

Current observed state:
- The original startup blocker in `js[2]` is gone
- With `OAB_TRACE_JS_ERRORS=1`, the remaining visible JS failure is now:
  - `js[3]` / `js[4]` ... `err=Unsupported member call: undefined.impl [via call mw.loader.impl]`
- Non-traced `inspect-page` still renders:
  - `final frames=5 text=444 image=1 svg=5`
- So the page is still visually unchanged relative to the prior fallback-driven state

Next goals:
1. Keep using instrumentation to identify the real first missing path behind `mw.loader.impl` instead of guessing
2. Determine whether the remaining failure is due to:
   - script ordering/execution model
   - dynamic script loading behavior
   - missing runtime/DOM behavior
   - or another global-state/resource-loader gap
3. Implement only the exact engine/runtime/DOM behavior needed for that blocker
4. Re-run:
   - `timeout 120s cargo test --lib`
   - `timeout 120s cargo run --bin inspect-page -- https://en.wikipedia.org/wiki/Riki_LeCotey --width 1366 --height 768 --timeout 20`
   - `timeout 120s env OAB_TRACE_JS_ERRORS=1 cargo run --bin inspect-page -- https://en.wikipedia.org/wiki/Riki_LeCotey --width 1366 --height 768 --timeout 20`
5. Re-verify against:
   - `tmp/compare/engine-riki-after-syntax-1366.png`
   - `tmp/oracle/riki-lecotey-20260321b/screenshot.png`
   - `tmp/oracle/riki-lecotey-20260321c/screenshot.png`
   - `tmp/oracle/riki-lecotey-20260321d/screenshot.png`

Notes:
- The branch is ahead of origin by 2 commits
- The last good commit before this one was `03173bf`
- Keep the work engine-first; no site-specific hacks and no third-party deps
