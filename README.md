# randomprime

This is a tool for randomizing/customizing the layout of pickups in Metroid Prime.
It has three parts: a webpage for generating a randomized layout [(Generator)](https://aprilwade.github.io/randomprime/generator.html), a webpage for editing layouts [(Editor)](https://aprilwade.github.io/randomprime/editor.html), and a command line program that modifies the game ISO [(ISO Patcher)](https://github.com/aprilwade/randomprime/releases/latest). All layouts created by the generator should be completable.

This system has no notion of a "seed".
Instead, a layout is communicated to and from these components as an 87 character long "layout descriptor".
To create a randomized ISO, you must first use the generator to get a layout descriptor and then run the ISO patcher using the layout descriptor.
Alternatively, you can create a layout from scratch using the editor.

## How to use the ISO patcher

If you're using Windows, you will need to use `cmd` to run the ISO patcher. Note, you can drag-and-drop files onto the `cmd` window rather than typing out their full path. An example:

```
"C:\Users\Me\Downloads\randomprime_patcher.exe" --input-iso="C:\Users\Me\mp1.iso" --output-iso="C:\Users\Me\mp1_random.iso" --layout=NCiq7nTAtTnqPcap9VMQk_o8Qj6ZjbPiOdYDB5tgtwL_f01-UpYklNGnL-gTu5IeVW3IoUiflH5LqNXB3wVEER4
```

If you would like to not have to play through the tutorial section (the Frigate) each time, you may also give the ISO patcher the ``--skip-frigate`` argument to skip it.

## Reporting a bug

If you believe that you have been generated uncompletable layout, please when filing an issue include the layout descriptor and the generator settings used.

If you experience a crash or some unexpected behavior while playing with a patched ISO, please be sure to include in your issue the layout descriptor, a hash of the input ISO, and a hash of the generated ISO.

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

##### Q: Why is there only one difficulty for the generator?
A:
Because I haven't gotten around to adding any others and its difficult to decide which capabilities/tricks should be required.

##### Q: Why don't you use seeds instead of this really long layout thingy?
A:
The advantage of this arrangement is that the inner-workings of the generator can be modified without users needing to re-download anything.
Also, this means pickup layouts can be customized or created from scratch using the editor if desired.

## To do

* A GUI for the ISO patcher
* Add an "Advanced" difficulty
* Randomize elevators (with completability guarantees) (someday)


## Thanks

The creation of this tool would not have been possible without the [Retro Modding Wiki](http://www.metroid2002.com/retromodding/wiki/Retro_Modding_Wiki) and amazing people who edit it.
Additionally, in many places where I wasn't sure how to do something (for example, skip item collection cutscenes) this tool's behavior emulates the randomizer created by [Claris Robyn](https://www.twitch.tv/clarisrobyn).
