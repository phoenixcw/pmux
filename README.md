# pmux
Path MUltipleXer — associate filesystem paths with tags, then list them back by tag.

## Install

### Homebrew (macOS and Linux)

```sh
brew tap phoenixcw/pmux
brew install pmux
```

### From source

```sh
cargo install --path .
```

## Usage

```sh
pmux add <tag> <path>   # associate path with tag (relative paths resolve against the current directory)
pmux list               # list all tags
pmux list <tag>         # list every path under that tag
```

All output is sorted lexicographically, and adding the same entry twice does not
create a duplicate. `pmux list <tag>` prints an error and exits with code 1 for an
unknown tag.

## Path handling

`add` stores a normalized absolute path: `~` is expanded, relative paths are
completed against the current directory, and `.`/`..` components are removed. When
the path already exists, `canonicalize` resolves symlinks as well. That way a given
directory is only ever recorded once.

## Storage location

The first available of the following is used:

1. `$PMUX_STORE`
2. `$XDG_DATA_HOME/pmux/store.json`
3. `~/.local/share/pmux/store.json`

The format is JSON. Writes go through a temporary file followed by a rename, so an
interruption never truncates existing data.

## Build

```sh
cargo build --release   # produces target/release/pmux
cargo test
```
