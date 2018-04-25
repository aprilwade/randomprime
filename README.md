# randomprime

[Download latest version](https://github.com/aprilwade/randomprime/releases/latest)

This is a program to randomize/customize the layout of pickups in Metroid Prime.
It does _not_ generate layouts, it merely patches the ISO.
You need a to use a separate utility to generate a "layout descriptor" that you feed to the patcher.
I recommend using [Bash's randomizer](https://etaylor8086.github.io/randomizer/).
I also wrote a [generator](https://aprilwade.github.io/randomprime/generator.html), but its not as featureful as Bash's.
There's an [editor webpage](https://aprilwade.github.io/randomprime/editor.html) that can be used to customize or create a layout.

## How to use the ISO patcher

If you're on Windows, you can launch the patcher by simply double clicking the EXE file in Explorer.
Alternatively, you can drag-and-drop your input ISO onto the EXE file to avoid manually typing its location later.

The patcher can also be run from a terminal.
If you run it without passing any arguments, it'll operate in interactive mode, just like when its launched from the GUI.
It also has a CLI, the details of which you can find by running it with the `-h` flag.

## Reporting a bug

If you file an issue, please include the layout descriptor you used, a hash of the input ISO, and a hash of the generated ISO.

## Faq

##### Q: Which versions of Metroid Prime are supported?
A: Only the NTSC original print run 0-00 version has been tested, because it's the only one I own.
The other two NTSCs versions (0-01 and 0-02 (Player's Choice)) may work, but no promises.
It is highly unlikely that any non-NTSC version will work.
The trilogy version is right out.
Hashes of a known good ISO dump are:
```
MD5:  737cbfe7230af3df047323a3185d7e57
SHA1: 1c8b27af7eed2d52e7f038ae41bb682c4f9d09b5
```

##### Q: Can a patched ISO be used as the input ISO?
A:
No, you must use a clean/unpatched input ISO.

##### Q: Why do I need a separate webpage to generate layouts?
A:
Because ~~I'm lazy~~ I wanted to allow other people to write their own generators or create layouts from scratch.

## To do

* A GUI


## Thanks

The creation of this tool would not have been possible without the [Retro Modding Wiki](http://www.metroid2002.com/retromodding/wiki/Retro_Modding_Wiki) and amazing people who edit it.
Additionally, in many places where I wasn't sure how to do something (for example, skip item collection cutscenes) this tool's behavior emulates the randomizer created by [Claris Robyn](https://www.twitch.tv/clarisrobyn).
