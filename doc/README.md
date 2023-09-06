# Building doc book

The book is built [using `mdbook`](https://rust-lang.github.io/mdBook/index.html).

To install everything you need, run the following:

```bash
cargo install --locked mdbook-variables mdbook
```

Note that at the time of this writing, you may see a warning message similar to: `Warning: The variables plugin was built against version 0.4.32 of mdbook, but we're being called from version 0.4.34`. This is a known issue from the mdbook-variables author. See here: https://gitlab.com/tglman/mdbook-variables/-/issues/3

Serve the book locally and open your default browser.

```bash
cd plrust/doc
mdbook serve --open
```