## xnotify

An Xorg-based program that shows an on-screen notification window with a simple text.

Similar to [osd_cat](http://manpages.ubuntu.com/manpages/lucid/man1/osd_cat.1.html), but with
some different features.

Originally in Python (2003), rewritten in Rust (2021).

Some code based on Chris Duerr's [leechbar](https://github.com/chrisduerr/leechbar).


### Invocation example

Appear in the bottom-right corner of the screen, flash for 5 seconds and disappear:

```
echo -n "I'm a tomato: 🍅" | xnotify -n "normal 30" -l -t 5 -p '%100,%100'
```

- If right click happens, disappears sooner.
- If dragged - cancels timeout.


### Syntax

```
xnotify 0.1.0

USAGE:
    xnotify [FLAGS] [OPTIONS]

FLAGS:
    -l, --blink      Make the window flash its colors
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --blink-duration <blink-duration>    Duration of the blink [default: 0.25]
    -e, --blink-rate <blink-rate>            Rate of the blink (time between each color flip) [default: 0.05]
    -n, --font <font>                        Font to use (Pango font string, for example "normal 100" for big text)
    -x, --from-file <from-file>              Take text from file instead of standard input. If file is '-', takes from
                                             standard input
    -p, --position <position>                Initial screen position [default: %50,%50]
    -t, --timeout <timeout>                  Time to wait until message automatically gets off the screen
```
