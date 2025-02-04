#!/usr/bin/env bash
## Post the Breaking Commit from Prometheus to Mastodon

set -e  ## Exit when any command fails

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

set -x  ## Echo commands

## Set the Prometheus Server
export PROMETHEUS_SERVER=luppys-mac-mini.local:9090

set +e  ## Ignore errors
for (( ; ; )); do
  ## Post the Breaking Commit from Prometheus to Mastodon
  cargo run
  break ####

  ## Wait a while
  date ; sleep 900
done
