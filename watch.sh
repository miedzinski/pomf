#!/usr/bin/env bash

inotifywait -m "$(xdg-user-dir PICTURES)" -e close_write |
    while read path action file; do
        "$(dirname $(readlink -f $0))/upload.py" "${path}${file}"
    done
