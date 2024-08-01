# fluent-bit-lsp

[![Server CI](https://github.com/sh-cho/vscode-fluent-bit/actions/workflows/server-ci.yaml/badge.svg?event=push)](https://github.com/sh-cho/vscode-fluent-bit/actions/workflows/server-ci.yaml)

[LSP(Language Server Protocol)](https://microsoft.github.io/language-server-protocol/) implementation
for [fluent-bit](https://fluentbit.io/) config

## [Server](./src)

`fluent-bit-language-server`

- language server framework: [tower-lsp](https://github.com/ebkalderon/tower-lsp)
- parser: [tree-sitter-fluentbit](https://github.com/sh-cho/tree-sitter-fluentbit)

## Clients

- [Visual Studio Code](./clients/vscode)
- nvim (TBD)

## Changelog

See [CHANGELOG.md](./CHANGELOG.md)

## How to contribute?

See [CONTRIBUTING.md](./CONTRIBUTING.md)

## License

Licensed under either of [Apache License Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
