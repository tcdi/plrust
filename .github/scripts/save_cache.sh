#!/usr/bin/env bash

# Creates cache archive and uploads to S3.
#
# Expects the following environment variables to already exist:
#  * ARTIFACT_USER_AWS_PROFILE: the profile to use when issuing AWS CLI commands
#  * AWS_CACHE_BUCKET: the S3 bucket in which to obtain the archive
#  * HOME: executing user's home directory
#
# Expects the following parameters (in-order):
#  * $1: the pre-calculated cache key
#  * $2: array of full-path directories to be cached
#
# Example usage:
#  . /path/to/plrust/.github/scripts/save_cache.sh
#  my_paths=(/path/one /path/two /path/three)
#  savecache "some-cache-key-abc123" "${my_paths[@]}"

function savecache() {
  local cache_key="$1"
  shift
  local dirs_to_save=("$@")

  echo "Checking to see if cache archive exists: $cache_key"

  if aws s3api head-object --profile $ARTIFACT_USER_AWS_PROFILE --bucket $AWS_CACHE_BUCKET --key $cache_key &> /dev/null; then
    echo "Cache archive exists for $cache_key -- skipping archive creation."
  else
    echo "Cache archive does not exist for $cache_key -- creating archive now."

    archive_path=$HOME/artifacts/$cache_key

    echo "Creating archive at $archive_path"

    tar --ignore-failed-read -cvf - "${dirs_to_save[@]}" \
      | lz4 - $archive_path

    echo "Created archive $archive_path -- uploading now"

    aws s3api put-object \
      --profile $ARTIFACT_USER_AWS_PROFILE \
      --bucket $AWS_CACHE_BUCKET \
      --key $cache_key \
      --body $archive_path

    echo "Removing $archive_path"
    rm $archive_path
  fi
}
