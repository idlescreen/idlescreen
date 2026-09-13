#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# $1 == 0 → erase (not upgrade). Schedule stack wipe after this transaction.
if [ "$1" -eq 0 ]; then
    if [ -x /usr/libexec/idlescreen/schedule-remove-stack ]; then
        /usr/libexec/idlescreen/schedule-remove-stack || true
    fi
fi
exit 0
