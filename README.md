![Auto-Rewind for Daily Test (Apache NuttX RTOS)](https://lupyuen.org/images/rewind-title.jpg)

# Apache NuttX RTOS: Notify via Mastodon the results of Rewind Builds

If the __Daily Test__ fails for [__Apache NuttX RTOS__](https://nuttx.apache.org/docs/latest/index.html)... Can we __Auto-Rewind__ and discover the __Breaking Commit__? Let's try this...

1.  Every Day at 00:00 UTC: __Ubuntu Cron__ shall trigger a __Daily Build and Test__ of NuttX for __QEMU RISC-V__ _(knsh64 / 64-bit Kernel Build)_

1.  __If The Test Fails:__ Our Machine will __Backtrack The Commits__, rebuilding and retesting each commit _(on QEMU Emulator)_

1.  When it discovers the __Breaking Commit__: Our Machine shall post a [__Mastodon Alert__](https://nuttx-feed.org/@nuttx_build/113922504467871604), that includes the _(suspicious)_ __Pull Request__

1.  __Bonus:__ The Machine will draft a [__Polite Note__](https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4801057) for our NuttX Colleague to investigate the Pull Request, please

_Why are we doing this?_

If NuttX Fails on __QEMU RISC-V__: High chance that NuttX will also fail on __RISC-V SBCs__ like Ox64 BL808 and Oz64 SG2000.

Thus it's important to Nip the Bud and Fix the Bug early, before it hurts our RISC-V Devs. _(Be Kind, Rewind!)_

# Find the Breaking Commit

We wrote a script that will __Rewind the NuttX Build__ and discover the Breaking Commit...

```bash
## Set the GitLab Token, check that it's OK
## export GITLAB_TOKEN=...
. $HOME/gitlab-token.sh
glab auth status

## Set the GitLab User and Repo for posting GitLab Snippets
export GITLAB_USER=lupyuen
export GITLAB_REPO=nuttx-build-log

## Download the NuttX Rewind Script
git clone https://github.com/lupyuen/nuttx-build-farm
cd nuttx-build-farm

## Find the Breaking Commit for QEMU RISC-V (64-bit Kernel Build)
nuttx_hash=  ## Optional: Begin with this NuttX Hash
apps_hash=   ## Optional: Begin with this Apps Hash
./rewind-build.sh \
  rv-virt:knsh64_test \
  $nuttx_hash \
  $apps_hash
```

Our Rewind Script runs __20 Iterations of Build + Test__...

```bash
## Build and Test: Latest NuttX Commit
git reset --hard HEAD
tools/configure.sh rv-virt:knsh64
make -j
qemu-system-riscv64 -kernel nuttx

## Build and Test: Previous NuttX Commit
git reset --hard HEAD~1
tools/configure.sh rv-virt:knsh64
make -j
qemu-system-riscv64 -kernel nuttx
...
## Build and Test: 20th NuttX Commit
git reset --hard HEAD~19
tools/configure.sh rv-virt:knsh64
make -j
qemu-system-riscv64 -kernel nuttx

## Roughly One Hour for 20 Rewinds of Build + Test
```

Then we run this script to Post the Breaking Commit from Prometheus to Mastodon: [run.sh](run.sh)

```bash
## Set the GitLab Token, User and Repo for posting GitLab Snippets
## export GITLAB_TOKEN=...
## export GITLAB_USER=lupyuen
## export GITLAB_REPO=nuttx-build-log
. $HOME/gitlab-token.sh

## Set the Mastodon Token
## export MASTODON_TOKEN=...
. $HOME/mastodon-token.sh

## Set the Rust Environment
. $HOME/.cargo/env

## Post the Breaking Commit from Prometheus to Mastodon
cargo run
```
