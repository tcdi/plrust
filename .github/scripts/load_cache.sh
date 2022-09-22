#!/usr/bin/env bash

# Downloads and extracts cache archive from S3.
#
# Expects the following environment variables to already exist:
#  * ARTIFACT_USER_AWS_PROFILE: the profile to use when issuing AWS CLI commands
#  * AWS_CACHE_BUCKET: the S3 bucket in which to obtain the archive
#  * HOME: executing user's home directory
#
# Expects the following parameters (in-order):
#  * $1: the pre-calculated cache key
#
# Example usage:
#  . /path/to/plrust/.github/scripts/load_cache.sh
#  loadcache "some-cache-key-abc123"

function loadcache() {
  local cache_key="$1"

  echo "Checking to see if cache archive exists: $cache_key"

  if aws s3api head-object --profile $ARTIFACT_USER_AWS_PROFILE --bucket $AWS_CACHE_BUCKET --key $cache_key &> /dev/null; then
    echo "Cache archive exists for $cache_key -- downloading and extracting now."

    mkdir -p $HOME/artifacts/
    archive_path=$HOME/artifacts/$cache_key

    echo "Downloadng archive $cache_key and storing to $archive_path"

    aws s3api get-object \
      --profile $ARTIFACT_USER_AWS_PROFILE \
      --bucket $AWS_CACHE_BUCKET \
      --key $cache_key \
      $archive_path

    echo "Extracting archive $cache_key"
    lz4 -dc --no-sparse $archive_path | tar xvC /

    echo "Removing archive $archive_path"
    rm $archive_path

    echo "Done."
  else
    echo "Cache archive does not exist for $cache_key -- skipping."
  fi
}
