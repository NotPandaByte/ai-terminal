.PHONY: build install install-local install-system uninstall uninstall-local uninstall-system clean

# Build release binary
build:
	cargo build --release

# Install to ~/.cargo/bin (default, where cargo binaries go)
install: build
	@mkdir -p ~/.cargo/bin
	@cp target/release/ai ~/.cargo/bin/ai
	@chmod +x ~/.cargo/bin/ai
	@echo "✅ Installed 'ai' to ~/.cargo/bin"
	@echo "You can now run: ai model, ai status, etc."

# Install to ~/.local/bin (alternative location)
install-local: build
	@mkdir -p ~/.local/bin
	@cp target/release/ai ~/.local/bin/ai
	@chmod +x ~/.local/bin/ai
	@echo "✅ Installed 'ai' to ~/.local/bin"
	@echo "You can now run: ai model, ai status, etc."

# Install to /usr/local/bin (system-wide, requires sudo)
install-system: build
	sudo cp target/release/ai /usr/local/bin/ai
	sudo chmod +x /usr/local/bin/ai
	@echo "✅ Installed 'ai' to /usr/local/bin (system-wide)"

# Uninstall from ~/.cargo/bin
uninstall:
	@rm -f ~/.cargo/bin/ai
	@echo "✅ Uninstalled 'ai' from ~/.cargo/bin"

# Uninstall from ~/.local/bin
uninstall-local:
	@rm -f ~/.local/bin/ai
	@echo "✅ Uninstalled 'ai' from ~/.local/bin"

# Uninstall from /usr/local/bin
uninstall-system:
	sudo rm -f /usr/local/bin/ai
	@echo "✅ Uninstalled 'ai' from /usr/local/bin"

# Clean build artifacts
clean:
	cargo clean

# Rebuild and reinstall (convenience target)
reinstall: clean install

