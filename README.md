# fluent-bit-lsp

[![Server CI](https://github.com/sh-cho/fluent-bit-lsp/actions/workflows/server-ci.yaml/badge.svg?event=push)](https://github.com/sh-cho/fluent-bit-lsp/actions/workflows/server-ci.yaml)

[LSP(Language Server Protocol)](https://microsoft.github.io/language-server-protocol/) implementation
for [fluent-bit](https://fluentbit.io/) config

> [!NOTE]
> This project is still in development and not fully-featured yet.

## Features

- Auto-completion for plugins
- Show documentation on hover
- Diagnostics

## [fluent-bit-language-server](./fluent-bit-language-server)

Language server implementation made
with [tower-lsp](https://github.com/ebkalderon/tower-lsp), [tree-sitter-fluentbit](https://github.com/sh-cho/tree-sitter-fluentbit)

## Clients

- [Visual Studio Code](./clients/vscode)
- nvim (TBD)
- helix (TBD)

## How to contribute?

Currently, this project is in the early stage of development and a lot of parts can be changed. So I don't think it's a good time to contribute to this project yet.

It doesn't mean that I don't accept contributions now, but I think it's better to wait until the project is more stable.

When it is ready, I will update this section. 🙏

## License

Licensed under either of [Apache License Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
