## xnotify

An Xorg-based program that shows an on-screen notification window with a simple text.

Similar to [osd_cat](http://manpages.ubuntu.com/manpages/lucid/man1/osd_cat.1.html), but with
some different features.

Originally in Python (2003), rewritten in Rust (2021).

Some code based on Chris Duerr's [leechbar](https://github.com/chrisduerr/leechbar).


### Invocation example

```
echo -n "I'm a tomato: 🍅" | cargo run --release -- -n "normal 30" -l
```

### Syntax

```
  xnotify [-hxtpfeidep] message

	 -h, --help                      This help screen.
	 -x file, --fromfile=file        Take text from file instead of standard input. If file is
									 '-', takes from standard input.
	 -t secs, --timeout=secs         Time to wait until message automatically gets off
									 the screen (default: 5)
	 -b color, --background=color    Color of the text's background (default: black)
	 -f color, --foreground=color    Color of the text's foreground (default: white)
	 -r color, --border=color        Color of the border (default: red)
	 -n font, --font=name            Name of the font to use (default: 9x15bold)

	 -l, --blink                     Make the window flash its colors (default: off)
	 -d secs, --blink-duration=secs  Duration of the blink.
	 -e secs, --blink-rate=secs      Rate of the blink (time between each color flip)

	 -p x,y  --position=x,y          Initial screen position (default: 100, 100)
```
