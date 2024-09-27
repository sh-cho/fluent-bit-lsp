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

- [Visual Studio Code](./clients/vscode) [![Visual Studio Marketplace](https://img.shields.io/badge/-Visual_Studio_Marketplace-007ACC)](https://marketplace.visualstudio.com/items?itemName=sh-cho.vscode-fluent-bit) [![Visual Studio Marketplace](https://img.shields.io/badge/-Open_VSX_Registry-A60EE5)](https://open-vsx.org/extension/sh-cho/vscode-fluent-bit)
- nvim (TBD)
- helix (TBD)

## How to contribute?

Currently, this project is in the early stage of development and a lot of parts can be changed. So I don't think it's a good time to contribute to this project yet.

It doesn't mean that I don't accept contributions now, but I think it's better to wait until the project is more stable.

When it is ready, I will update this section. 🙏

## License

Licensed under either of [Apache License Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.


<!-- Analytics -->
<img referrerpolicy="no-referrer-when-downgrade" src="https://static.scarf.sh/a.png?x-pxid=5616ddac-9734-42c7-a7c1-43da139f146f" />
