#!/bin/sh

if [ -z "$_FUSE_COMMFD" ]; then
    FD_ARGS=
else
    FD_ARGS="--env=_FUSE_COMMFD=${_FUSE_COMMFD} --forward-fd=${_FUSE_COMMFD}"
fi

if [ -e /proc/self/fd/3 ] && [ 3 != "$_FUSE_COMMFD" ]; then
    FD_ARGS="$FD_ARGS --forward-fd=3"
fi

# If the fusermount3 binary doesn't exist we try fusermount
# command -v will return 0 if the command exists and 127 otherwise
if flatpak-spawn --host sh -c "command -v fusermount3" > /dev/null; then
    exec flatpak-spawn --host --forward-fd=1 --forward-fd=2 $FD_ARGS fusermount3 "$@"
else
    exec flatpak-spawn --host --forward-fd=1 --forward-fd=2 $FD_ARGS fusermount "$@"
fi
