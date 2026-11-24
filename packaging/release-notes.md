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

On its first start, the app offers to install what bots need: a C compiler and a private
copy of Rust, about 1 GB. Nothing is installed without asking.
