#!/usr/bin/env python3

import pathlib
import re
import subprocess
import sys

import requests


UPLOAD_URL = 'https://pomf.cat/upload.php'
FILE_URL = 'https://a.pomf.cat/{filename}'
FILE_PATTERN = 'Screenshot_.*'


def main():
    path = pathlib.Path(sys.argv[1])

    if not path.exists():
        return

    if not re.match(FILE_PATTERN, path.name):
        return

    with path.open('rb') as f:
        resp = requests.post(UPLOAD_URL, files={'files[]': f})

    if 200 <= resp.status_code < 300 and resp.json().get('success'):
        filename = resp.json()['files'][0]['url']
        url = FILE_URL.format(filename=filename)
        subprocess.run(['xclip', '-i'], input=url.encode())


if __name__ == '__main__':
    main()
