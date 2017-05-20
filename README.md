Quick and dirty scripts to auto-upload screenshots and copy URL to clipboard.

I use it with KDE spectacle (with appropriate keyboard shortcuts set):

- full screen
```
spectacle --background --fullscreen
```

- active window
```
spectacle --background --activewindow
```

- region
```
spectacle --background --region
```

By default it saves pictures to `$XDG_PICTURES_DIR`,
which is watched by `watch.sh`. `upload.py` takes care of the rest.

If you want to try it, simply execute `watch.sh`.

Requires Python 3 with `requests` library, `inotify-tools` and `xclip`.
