<h1 align="center">shvl</h1>

<p align="center">
  A small CLI for organizing NixOS packages into groups.
</p>

---

## About

`shvl` exists to make multi-system NixOS configurations easier to manage. Once a
single flake builds a laptop, a desktop and a server, the package lists are the
part that gets unwieldy first: every host wants a slightly different set,
`environment.systemPackages` grows into one long list per machine, and the
overlap between them ends up duplicated or buried behind ad hoc `mkIf` and
`optionals` conditions.

Instead, `shvl` manages a directory of Nix package lists, called groups. Each
group is a `.nix` file that evaluates to a list of packages, so it can be
imported straight into a NixOS configuration. A host picks the groups it wants,
and shared groups are written once and imported by every machine that needs
them.

Groups live under a `.shvl` directory and can be nested, letting you organize
packages however you like without resorting to long prefixed names. Adding a
package to a group updates every host that imports it, and the CLI keeps you
from hand-editing lists spread across several host files.

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

### Examples

```
shvl group create editors/neovim
shvl add ripgrep -g editors/neovim
shvl group info editors/neovim
shvl group list
shvl group remove editors/neovim
```

## Building

### Cargo

```
cargo build --release
```

Needs a C linker on `PATH`; on NixOS run it inside `nix develop`.

### Nix

```
nix develop          # dev shell, adds rust-analyzer, rustfmt, clippy
nix build            # flakes
nix-build            # without flakes
```

### As a flake input

Exposes `packages.<system>.default` for `x86_64-linux` and `aarch64-linux`.

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    shvl.url = "github:<owner>/shvl";
    shvl.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, shvl, ... }:
    {
      nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ./configuration.nix
          ({ pkgs, ... }: {
            environment.systemPackages = [ shvl.packages.${pkgs.system}.default ];
          })
        ];
      };
    };
}
```

## Group files

A group file is a function that takes `pkgs` and returns a list of packages.
`shvl group create editors/neovim` followed by `shvl add neovim -g
editors/neovim` and `shvl add ripgrep -g editors/neovim` writes
`.shvl/editors/neovim.nix`:

```nix
# This file is autogenerate by shvl. Do no modify unless you know what you are doing

pkgs:
(
	with pkgs;
	[
		neovim
		ripgrep
	]
)
```

Packages are kept sorted, and attribute paths such as `nodePackages.prettier`
are preserved as written.

## Using groups in a NixOS config

Because a group is just a function of `pkgs`, importing it and applying `pkgs`
yields a package list you can splice into `environment.systemPackages`:

```nix
{ pkgs, ... }:

{
  environment.systemPackages =
    import ./.shvl/base.nix pkgs
    ++ import ./.shvl/editors/neovim.nix pkgs;
}
```

The same works for `users.users.<name>.packages` or a Home Manager
`home.packages`, since all of them take a list of packages.

### Multiple systems

With more than one host, a small helper turns a host's config into a list of the
group names it wants:

```nix
{ pkgs, ... }:

let
  group = name: import (./.shvl + "/${name}.nix") pkgs;
in
{
  environment.systemPackages = builtins.concatMap group [
    "base"
    "editors/neovim"
    "desktop"
  ];
}
```

A server then differs from a desktop only in that list, swapping `desktop` for
`server` while still sharing `base` and `editors/neovim`. Running `shvl add
ripgrep -g base` updates every host that lists `base` on its next rebuild, with
no per-host edits.

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
