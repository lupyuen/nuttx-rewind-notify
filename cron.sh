#!/usr/bin/env bash
## Cron Job: Post the Breaking Commit from Prometheus to Mastodon

set -e  ## Exit when any command fails

## Set the GitLab Token
## export GITLAB_TOKEN=...
. $HOME/gitlab-token.sh

## Set the Mastodon Token
## export MASTODON_TOKEN=...
. $HOME/mastodon-token.sh

## Set the Rust Environment
. $HOME/.cargo/env

set -x  ## Echo commands

## Set the Prometheus Server
export PROMETHEUS_SERVER=luppys-mac-mini.local:9090

## Get the Script Directory
script_path="${BASH_SOURCE}"
script_dir="$(cd -P "$(dirname -- "${script_path}")" >/dev/null 2>&1 && pwd)"

## Post the Breaking Commit from Prometheus to Mastodon
cd $script_dir
cargo run
