# Agent extras (full-scope agents only)

Read after AGENTS.md when you own a whole phase (Grok Build or Grok Bot). The local model skips this.

## Project context
- Goal: feature parity with gif_creator_v2, minimal deps, shipped as user Flatpak `dev.nic.GifCreator`
  (GNOME 50). Layout also has `tools/v2-golden/` (C++ harness dumping v2 reference frames, never shipped).
- Read `documentation/PLAN.md` for the milestone you are given; keep designs compatible with later
  ones. `documentation/FEATURE_INVENTORY.md` is the parity checklist.
- Exported GIFs must be byte-identical for any thread count.

## Changelog (hard rule)
Every user-visible change gets an entry under `[Unreleased]` in `documentation/CHANGELOG.md` in the
same change (Keep a Changelog: Added/Changed/Removed/Fixed, each at most once per section).

## Commits
Commit to `main` with conventional messages (`feat(core): …`, `fix(export): …`, `test: …`,
`docs: …`, `chore: …`). Never push.

## Docs
Topic docs live in `documentation/` as `UPPER_SNAKE.md`; deferred work goes in `documentation/TODO.md`
(High/Medium/Low). Back up before a major doc rewrite to `backups/<topic>-YYYY-MM-DD/` (gitignored).

## Runs
Per milestone run only: the Flatpak build, `cargo test --workspace --locked`, clippy, fmt. No GUI or
pointer automation, no screenshots. Nic tests by hand from `documentation/TEST_PLAN.md`; add rows for
anything you could not check. One failed approach, then stop and report.

## Flatpak build
```sh
cd /home/nic/Software/gif_creator_rs
flatpak run --command=flatpak-builder-wrapper org.flatpak.Builder \
    --user --install --force-clean --install-deps-from=flathub \
    flatpak-build dev.nic.GifCreator.yml
```
After any `Cargo.lock` change, regenerate `cargo-sources.json` with
[flatpak-cargo-generator.py](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo):
`python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json`.
The host rustup toolchain is too old and `~/.cargo/bin` has a stale cargo-clippy; always use the SDK
commands from AGENTS.md. Core tests are faster with `--release`.

## Report back
Commits (hash + message), what changed, build/test/clippy results, the changelog entry, and anything
skipped, guessed or unverified.
