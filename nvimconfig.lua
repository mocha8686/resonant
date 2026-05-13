local function devServer()
	local toggleterm = require("toggleterm")
	toggleterm.exec("bacon pedantic", 1)
	toggleterm.toggle(2)
	vim.notify("Dev environment started.", vim.log.levels.INFO, { title = "nvimconfig" })
end

vim.keymap.set("n", "<leader>td", devServer, { noremap = true, silent = true, desc = "Open dev environment" })
