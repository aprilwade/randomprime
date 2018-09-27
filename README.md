# randomprime

**A GUI frontend for this program is available [here](https://randomizer.metroidprime.run).** I highly recommend using it.

[Download latest version](https://github.com/aprilwade/randomprime/releases/latest)

This is a program to randomize/customize the layout of pickups in Metroid Prime.
It does _not_ generate layouts, it merely patches the ISO.
You need a to use a separate utility to generate a "layout descriptor" that you feed to the patcher.
There is a layout generator built into the aforementioned GUI.
I've also written a [generator webpage](https://aprilwade.github.io/randomprime/generator.html), though it is less featureful.
There's an [editor webpage](https://aprilwade.github.io/randomprime/editor.html) that can be used to customize or create a layout.

## How to use the ISO patcher

If you're on Windows, you can launch the patcher by simply double clicking the EXE file in Explorer.
Alternatively, you can drag-and-drop your input ISO onto the EXE file to avoid manually typing its location later.

The patcher can also be run from a terminal.
If you run it without passing any arguments, it'll operate in interactive mode, just like when its launched from the GUI.
The patcher also has a CLI, the details of which you can find by running it with the `-h` flag.

## Reporting a bug

If you file an issue, please include the layout descriptor you used, a hash of the input ISO, and a hash of the generated ISO.

## Faq

##### Q: Which versions of Metroid Prime are supported?
A:
Only the NTSC 0-00 and 0-02 (aka 1.00 and 1.02) versions are supported.
The 00-1 NTSC version, non-NTSC versions and the trilogy version will not work.
Hashes of a known good 0-00 ISO dump are:
```
MD5:  eeacd0ced8e2bae491eca14f141a4b7c
SHA1: ac20c744db18fdf0339f37945e880708fd317231
```

##### Q: Can a patched ISO be used as the input ISO?
A:
No, you must use a clean/unpatched input ISO.

##### Q: Why do I need a separate webpage to generate layouts?
A:
Because ~~I'm lazy~~ I wanted to allow other people to write their own generators or create layouts from scratch.

## To do

* Support Prime 2???

## Thanks

The creation of this tool would not have been possible without the [Retro Modding Wiki](http://www.metroid2002.com/retromodding/wiki/Retro_Modding_Wiki) and amazing people who edit it.
Additionally, in many places where I wasn't sure how to do something (for example, skip item collection cutscenes) this tool's behavior emulates the randomizer created by [Claris Robyn](https://www.twitch.tv/clarisrobyn).
