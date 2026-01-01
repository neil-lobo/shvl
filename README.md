<h1 align="center">shvl</h1>

<p align="center">
  A small CLI for organizing NixOS packages into groups.
</p>

---

## About

`shvl` manages a directory of Nix package lists, called groups. Each group is a
`.nix` file that evaluates to a list of packages, so it can be imported straight
into a NixOS configuration.

Groups live under a `.shvl` directory and can be nested, letting you organize
packages however you like without resorting to long prefixed names.

## Usage

### Packages

```
shvl add <package> [-g <group>]
shvl remove <package> [-g <group>]
```

`remove` is also available as `rm`. If `-g` is omitted, an interactive fuzzy
selector opens so you can pick a group.

### Groups

```
shvl group create <group>
shvl group info <group>
shvl group remove <group>
shvl group list
```

### Options

```
-g, --group <group>    Target group. Prompts interactively when omitted.
-d, --dir <dir>        Override the .shvl directory for a single command.
```

### Examples

```
shvl group create editors/neovim
shvl add ripgrep -g editors/neovim
shvl group info editors/neovim
shvl group list
shvl group remove editors/neovim
```

## The .shvl directory

The directory is resolved in this order:

1. The `--dir` flag, if given.
2. The path stored in `$HOME/.local/share/shvl/dir`, if that file exists.
3. `/etc/nixos/.shvl`.

## Group names

Group names are `/` separated paths relative to the `.shvl` directory. The final
segment names the `.nix` file and any preceding segments name subdirectories.

```
base             ->  .shvl/base.nix
editors/neovim   ->  .shvl/editors/neovim.nix
lang/rust/tools  ->  .shvl/lang/rust/tools.nix
```

Intermediate directories are created as needed, and removed again once the last
group inside them is removed. Group names are always shown and accepted in their
full `/` separated form.

Names must follow these rules:

- The name cannot be empty.
- The name cannot begin or end with `/`.
- No segment may be empty, so `a//b` is rejected.
- No segment may be `.` or `..`.
- No segment may begin with `.`.
- No segment may contain `\` or control characters.
