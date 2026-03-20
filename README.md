# dalsoop-tmux-tools

A collection of tmux utilities built in Rust.

## Tools

| Crate | Description |
|-------|-------------|
| [tmux-sessionbar](crates/tmux-sessionbar/) | Clickable session list for the tmux status bar |

## Requirements

- tmux >= 3.4
- Rust >= 1.70

## Install

```bash
git clone https://github.com/dalsoop/dalsoop-tmux-tools.git
cd dalsoop-tmux-tools
cargo build --release
```

Binaries will be in `target/release/`.

## License

MIT
