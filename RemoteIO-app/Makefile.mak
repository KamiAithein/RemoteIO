# https://superuser.com/questions/1049430/how-do-you-set-environment-variables-for-a-single-command-on-windows
cargo-run:
	cmd /C "set LIBCLANG_PATH=c-src" && cargo run

.PHONY: cargo-run