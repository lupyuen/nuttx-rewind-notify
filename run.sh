#!/usr/bin/env bash
## Post the Breaking Commit from Prometheus to Mastodon

set -e  ## Exit when any command fails
set -x  ## Echo commands

## Set the Access Tokens for Mastodon and GitLab
## https://docs.joinmastodon.org/client/authorized/#token
## export MASTODON_TOKEN=...
## export GITLAB_TOKEN=...
set +x  ## Disable Echo
. ../mastodon-token.sh
. ../gitlab-token.sh
set -x  ## Echo commands

set +e  ## Ignore errors
for (( ; ; )); do
  ## Post the Breaking Commit from Prometheus to Mastodon
  cargo run
  break ####

  ## Wait a while
  date ; sleep 900
done
