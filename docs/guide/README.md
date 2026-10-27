# The user guide

The chapters are Markdown (`NN-name.md`), listed with their groups in `contents.json`. They are published on the
project page as `guide/` (an index and one page per chapter).

The pictures are made by the acceptance checks, from a real window in the nested compositor:

- `guide-sheets` types each `sheets/NAME.txt` into a fresh sheet on the fixed clock (2024-06-15 12:00 UTC) and
  keeps `docs/shots/guide/NAME.png`. It fails if a line that is not meant to be an error is one, so the guide
  never pictures a sheet that does not work. A name containing `error` is exempt. A first line `@width N` or
  `@size WIDTH HEIGHT` (points) overrides the default window.
- `guide-screens` takes the window, sidebar, menus, name list and Settings pictures.

To rebuild the pages and pictures for the site:

```sh
BABBAGE_LOWPRIO= bin/accept guide-sheets guide-screens
python3 tools/build_guide.py docs/guide ../jrimmer.github.io/babbage/guide docs/shots/guide
```

Every example in the chapters was run through the engine; write the answer the engine gives, and check a new one
the same way before adding it.
