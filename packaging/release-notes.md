**Pre-alpha.** RustBot Creator works, but expect rough edges and changes. Please report
problems in the [issues](https://github.com/kotoanidev/rustbot-creator/issues).

## Install

Fedora 43 and 44 (x86_64):

```sh
sudo dnf copr enable kotoanidev/rustbot-creator
sudo dnf install rustbot-creator
```

Ubuntu 24.04 and 26.04 (x86_64):

```sh
curl -fsSL https://kotoanidev.github.io/rustbot-creator/apt/rustbot-creator.gpg \
  | sudo tee /usr/share/keyrings/rustbot-creator.gpg > /dev/null
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/rustbot-creator.gpg] https://kotoanidev.github.io/rustbot-creator/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/rustbot-creator.list
sudo apt update && sudo apt install rustbot-creator
```

Updates then come through dnf or apt. The .deb is also attached below.

Windows 10 and 11 (x64): run `rustbot-creator-…-windows-x86_64-setup.exe` from the files
below. It installs for your user only, without an administrator. This pre-alpha is not
code-signed yet, so Windows SmartScreen warns about it: choose "More info", then "Run
anyway". Later it also comes through winget.

On its first start, the app offers to install what bots need: a C compiler and a private
copy of Rust, about 1 GB. Nothing is installed without asking.
