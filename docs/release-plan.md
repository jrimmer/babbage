# Release plan for 1.0

Nothing here has been done. Every step that reaches outside this machine needs the maintainer's yes.

## 1. Before a repository goes public

- Read `git log -p` of the cleaned tree for secrets and personal details: keys, tokens, addresses, host names of
  private machines (`git.example.com`, `buildhost`), and the maintainer's paths. `docs/overnight-handoff.md`, `docs/goal.md`
  and `docs/evidence/` name the build machine; decide whether to keep them in the public tree.
- Publish from a fresh one-commit history of the cleaned tree, so the private history (which holds the working
  notes and the earlier test data) stays private: `git checkout --orphan public && git commit`, then push that branch
  as `main` to the new repository.
- The project page already links to `github.com/jrimmer/babbage`, its releases and `crates.io/crates/babbage-core`.
  Those links 404 until the repository, the first release and the crate exist, so publish them first or take the
  links off the page.
- Check the tests for data copied from another product's documentation (the earlier pass removed it; run
  `grep -rIn` for the product's name and for long verbatim tables before the history is cut).

## 2. The release files

- Version: every crate says 0.0.1; set 1.0.0 in the app and core `Cargo.toml` files, tag `v1.0.0`; the core crate is
  tagged `core-v1.0.0`.
- `bin/package` builds `dist/babbage-1.0.0-x86_64.tar.gz` and its `.sha256`, `babbage-version.txt` and its
  `.sha256`, and the AppImage when `appimagetool` is on PATH. The icon is `packaging/babbage.svg` and the desktop entry is written
  by the script. The AppImage was built and run on buildhost (`bbg` mode, checksum verified) on 2026-10-27.
- Upload `install.sh` (with its `.sha256`) beside them: the one-line install is
  `curl -fsSL https://github.com/jrimmer/babbage/releases/latest/download/install.sh | bash`.
- Release notes: the 1.0.0 entry of `CHANGELOG.md`, then the short network statement from `docs/updates.md`.

## 3. The crate

`babbage-core` packages on its own (`cargo package -p babbage-core`). Publishing is `cargo publish -p babbage-core`
after the repository is public and the maintainer has said yes. It cannot be unpublished, only yanked.

## 4. The page

Rebuild the guide (`python3 tools/build_guide.py docs/guide ../jrimmer.github.io/babbage/guide docs/shots/guide`),
retake the pictures with `bin/accept guide-sheets guide-screens showcase`, and push the site repository.

## 5. After 1.0

An Omarchy overlay on `bbg --json`; rate, price and weather lookups (tickets 0003 to 0010); a shipped holiday list.
