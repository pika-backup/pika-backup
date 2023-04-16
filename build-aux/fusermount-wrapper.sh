#!/bin/sh

if [ -z "$_FUSE_COMMFD" ]; then
    FD_ARGS=
else
    FD_ARGS="--env=_FUSE_COMMFD=${_FUSE_COMMFD} --forward-fd=${_FUSE_COMMFD}"
fi

if [ -e /proc/self/fd/3 ] && [ 3 != "$_FUSE_COMMFD" ]; then
    FD_ARGS="$FD_ARGS --forward-fd=3"
fi

# Ignore stdout, redirect stderr for further inspection
STDERR="$(exec flatpak-spawn --host --forward-fd=1 --forward-fd=2 $FD_ARGS fusermount "$@" 2>&1 > /dev/null)"
RETURN=$?

# If the fusermount binary doesn't exist we try fusermount3
if [[ "$RETURN" -eq 1 && "$STDERR" == *"No such file or directory"* ]]; then
    STDERR="$(exec flatpak-spawn --host --forward-fd=1 --forward-fd=2 $FD_ARGS fusermount3 "$@" 2>&1 > /dev/null)"
    RETURN=$?
fi

# Output stderr from the last fusermount call for further inspection
echo -n "$STDERR" >&2

# Return the return code of the last fusermount call
exit $RETURN