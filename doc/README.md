# Building doc book

The book is built [using `mdbook`](https://rust-lang.github.io/mdBook/index.html).

Install mdbook -- exact version is required since (at the time of this writing) there were some compatibility issues.

```bash
cargo install mdbook --version 0.4.32
```

Install mdbook-variables preprocessor -- exact version is required since (at the time of this writing) there were some compatibility issues.

```bash
cargo install mdbook-variables --version 0.2.1
```

Serve the book locally and open your default browser.

```bash
cd plrust/doc
mdbook serve --open
```