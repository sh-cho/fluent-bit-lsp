local id = vim.lsp.start_client({
    name = 'fluent-bit-language-server',
    -- cmd = { 'fluent-bit-language-server' },
    cmd = { '/Users/seonghyeoncho/Desktop/my/github/fluent-bit-lsp/target/debug/fluent-bit-language-server'},
    root_dir = vim.fs.dirname(
        vim.fs.find({ 'Cargo.toml' }, { upward = true })[1]
    ),
})

vim.api.nvim_create_autocmd("BufNew", {
    callback = function(args)
        vim.lsp.buf_attach_client(args.buffer, id);
    end,
});

vim.api.nvim_create_autocmd('LspAttach', {
    callback = function(args)
        print(vim.inspect(args));
    end,
})

