# Public examples

Original examples live in `tests/fixtures/python`, `rust`, `javascript` and
`typescript`. Each language directory is a separate analysis root.
`calls.expected.json` contains manually annotated distinct call pairs; dynamic
calls deliberately remain unresolved.

Run `glyphibe analyze tests/fixtures/python`, then `glyphibe --repo
tests/fixtures/python search calculate`. Open that folder in the desktop to
explore its graph and teaching. Glyphibe parses these examples as data and
never executes them.
