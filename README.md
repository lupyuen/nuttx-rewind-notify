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


# Be Kind, Rewind!

1.  _Wow this looks super complicated. Does it work?_

    Dunno, we're still testing? Hopefully the New System will make my __Daily Routine__ a little less painful...

    - Every Morning: I check the [__NuttX Daily Test__](https://github.com/lupyuen/nuttx-riscv64/releases/tag/qemu-riscv-knsh64-2025-01-12)

    - Oops Daily Test failed! I run a script to [__Rewind or Bisect__](https://github.com/lupyuen/nuttx-riscv64/blob/main/special-qemu-riscv-knsh64.sh#L45-L61) the Daily Build

    - I write a [__Polite Note__](https://github.com/apache/nuttx/pull/15444#issuecomment-2585595498) _(depending on my mood)_

    - And post it to the __Breaking Pull Request__

    That's why we're __Fast Tracking__ the complicated new system: Right now it runs __Every Hour__ (instead of every day)

1.  _What if it's a smashing success?_

    We might extend the __Daily Rewind__ to a Real Board: [__Oz64 SG2000 RISC-V SBC__](https://lupyuen.github.io/articles/sg2000a).

    Or maybe [__SG2000 Emulator__](https://lupyuen.github.io/articles/sg2000b) and [__Ox64 Emulator__](https://lupyuen.github.io/articles/tinyemu3), since they're quicker and more consistent than Real Hardware. (Though less accurate)

    Plus other __QEMU Targets__: _rv-virt:nsh / nsh64 / knsh_

1.  _Suppose we wish to add Our Own Boards to the System?_

    Let's assume we have __Automated Board Testing__. Then we could upload the __NuttX Test Logs__ _(in the prescribed format)_ to GitLab Snippets or GitHub Gists. They'll appear in NuttX Dashboard and Build History.

    (Rewinding the Build on Our Own Boards? Needs more work)

1.  _Why Rewind every commit? Isn't Git Bisect quicker?_

    Ah remember that we're fixing Runtime Bugs, not Compile Errors. Git Bisect won't work if the Runtime Bug is [__Not Reliably Reproducible__](https://lupyuen.github.io/articles/bisect#good-commit-goes-bad).

    When we Rewind 20 Commits, we'll know if the bug is Reliably Reproducible.

1.  _Why aren't we using Docker?_

    Docker doesn't run OSTest correctly on [__QEMU RISC-V 64-bit__](https://lupyuen.github.io/articles/rust6#appendix-nuttx-qemu-risc-v-fails-on-github-actions).

1.  _Any more Grand Plans?_

    We might allow a __PR Comment__ to trigger a Build + Test on QEMU. For example, this PR Comment...

    ```bash
    @nuttxpr test rv-virt:knsh64
    ```

    Will trigger our __Test Bot__ to download the PR Code, and run Build + Test on QEMU RISC-V. Or on __Real Hardware__...

    ```bash
    @nuttxpr test milkv_duos:nsh
    ```
    
    Super helpful for __Testing Pull Requests__ before Merging. But might have [__Security Implications__](https://github.com/apache/nuttx/issues/15731#issuecomment-2628647886) 🤔

![Daily Test + Rewind is hosted on this hefty Ubuntu Xeon Workstation](https://lupyuen.github.io/images/ci4-thinkstation.jpg)

<span style="font-size:80%">

[_Daily Test + Rewind is hosted on this hefty Ubuntu Xeon Workstation_](https://qoto.org/@lupyuen/113517788288458811)

</span>
