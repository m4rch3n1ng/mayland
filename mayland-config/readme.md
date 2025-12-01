
### mayland-config

mayland uses the [mayfig](https://github.com/m4rch3n1ng/mayfig) language for configuration. you can take a look there
to find an explanation of the syntax.  
an example config can be found at [mayland.mf](https://github.com/m4rch3n1ng/mayland/tree/main/resources/mayland.mf).

the config is located at `~/.config/mayland.mf`. if no config is found, mayland will populate that path on startup.

**table of content**
- [`input {}`](#input)
- [`output {}`](#output)
- [`cursor {}`](#cursor)
- [`decoration {}`](#decoration)
- [`layout {}`](#layout)
- [`env {}`](#env)
- [`bind {}`](#bind)
- [`windowrules {}`](#windowrules)

#### input

you can configure your input in the `input` category in your `mayland.mf` file.

```ini
input {
    keyboard {
        # xkb-file = "~/.config/keymap/may.xkb"

        # xkb-layout = "de,us"
        # xkb-model = "thinkpad"
        # xkb-variant = "nodeadkeys"
        # xkb-options = "ctrl:nocaps"

        repeat-delay = 600
        repeat-rate = 25
    }

    touchpad {
        tap = true
        tap-and-drag = false
        tap-drag-lock = false

        dwt = true
        dwtp = true

        natural-scroll = true
        # scroll-method = "two-finger"

        # click-method = "clickfinger"

        middle-emulation = true
        tap-button-map = "left-right-middle"
        left-handed = false

        accel-speed = 0
        # accel-profile = "adaptive"
    }

    mouse {
        natural-scroll = false

        middle-emulation = false
        left-handed = false

        accel-speed = 0
        # accel-profile = "adaptive"
    }

    tablet {
        map-to = "all"
        relative = false
    }
}
```

> [!TIP]
>
> you can also set device-specific configurations for touchpads, mice and tablets
>
> ```ini
> input {
>     mouse [ "<some gaming mouse>" ] {
>         accel-profile = "flat"
>     }
> }
> ```
>
> you can run `mayctl devices` inside of mayland to get the available devices and their names.
>
> *this is not yet supported for keyboards*

#### keyboard

##### xkb

you can set your keyboard layout, model, variant and options in the keyboard section. these are passed directly to xkbcommon.
you can find out more on the xkeyboard-config(7) manpage.

```ini
input {
    keyboard {
        xkb-layout = "de,us"
        xkb-model = "thinkpad"
        xkb-variant = "nodeadkeys"
        xkb-options = "ctrl:nocaps"
    }
}
```

alternatively you can set a path directly to a .xkb file containing an xkb keymap. this overrides all other xkb settings.

```ini
input {
    keyboard {
        xkb-file = "~/.config/keymap/may.xkb"
    }
}
```

##### repeat

you can change the key repeat config.

`repeat-delay` changes the delay in ms before the key starts repeating
`repeat-rate` changes the rate of repeated key preses per second

```ini
input {
    keyboard {
        repeat-delay = 600
        repeat-rate = 25
    }
}
```

#### pointing devices

the pointing devices `touchpad` and `mouse` share some of their config.

these include:

- `natural-scroll`, which inverts the scrolling direction if set to true. defaults to `true` for touchpads and `false` for mice.
- `accel-speed` controls the linear acceleration speed. accepts values between `-1.0` and `1.0`, and defaults to `0.0`.
- `accel-profile` controls the [pointer acceleration profile](https://wayland.freedesktop.org/libinput/doc/latest/pointer-acceleration.html#pointer-acceleration-profiles). can be set to `"adaptive"` (the default) and `"flat"` (disables pointer acceleration).
- `middle-emulation`. emulate a middle mouse click by pressing left and right at the same time. defaults to `true` for touchpads and `false` for mice.
- `left-handed` enables left-handed mode, which inverts left and right click. off by default.

settings specific to touchpads are:

- `tap` enables / disables tapping support. on by default.
- `tap-and-drag` enables the libinput [tap-and-drag](https://wayland.freedesktop.org/libinput/doc/latest/tapping.html#tap-and-drag) setting.
- `tap-drag-lock` controls the drag lock for `tap-and-drag`. both of these are off by default.
- `dwt` disables the touchpad while typing. defaults to `true`.
- `dwtp` disables the touchpad while trackpointing. defaults to `true`.

#### tablets

```ini
input {
    tablet {
        map-to = "all"
        relative = false
    }
}
```

the currently available tablet configs are:

- `map-to`, which controls what the tablet maps to. currently available options are `"all"`, which maps the tablet across all outputs, `"active"` to map it to the current output and `"output" [ "<name>" ]`, which maps it to that output. defaults to `"all"`.
- `relative` makes the tablet motions relative. defaults to `false`.

### output

you can configure your outputs in the `output` category. each outputs get their own key, which is (as of right now) the connector.
you can list your outputs by executing `mayctl outputs` inside of mayland.

```ini
output {
    "e-DP1" {
        mode = "2256x1504@60"
        active = true
        position = [ 0 0 ]
    }
}
```

#### mode

sets the output resolution and refresh rate.

the format is `"<width>x<height>"` and allows you to optionally pass the refresh rate as `"<width>x<height>@<refresh-rate in hz>"`.
if the refresh rate is omitted, mayland will pick the mode with the highest refresh rate,
otherwise it will pick the refresh rate that matches the given one as closely as possible.

if the mode is not set at all or the resolution does not exist, mayland will pick the preferred mode with the highest quality.

running `mayctl outputs` inside of mayland will list all outputs and all their available modes.

### cursor

you can configure your cursor theme and size in the `cursor` category.
mayland currently only accepts cursors in the xcursor format.

```ini
cursor {
    xcursor-theme = "Bibata-Modern-Classic"
    xcursor-size = 24
}
```

if `xcursor-theme` or `xcursor-size` is not set, mayland will attempt to read their values from the `XCURSOR_THEME` and `XCURSOR_SIZE`
environment variables and will otherwise fallback to `"default"` and `24`.

### decoration

you can configure mayland decorations.

```ini
decoration {
    background = "#008080"

    focus {
        active = "#a21caf"
        inactive = "#71717a"
        thickness = 4
    }
}
```

#### background

you can set your background to a solid color in the config.

```ini
decoration {
    background = "#008080"
}
```

mayland additionally supports the `wlr-layer-shell` wayland protocol, which means that you can use things
like `swww` or `hyprpaper` or most other wayland "wallpaper engines" in mayland as well.

#### focus ring

the focus ring is the border drawn around windows. you can configure its thickness (in px), its color
when the window is active and its color when the window is inactive.

```ini
decoration {
    focus {
        active = "#a21caf"
        inactive = "#71717a"
        thickness = 4
    }
}
```

### layout

you can configure how mayland layouts your windows in the `layout` category.

### tiling

in the nested `tiling` you can configure the mayland tiling layout.

```ini
layout {
    tiling {
        gaps = 10
        border = 20
    }
}
```

available tiling layout options are:

- `gaps`, which sets the gap (in px) mayland leaves between two tiled windows. defaults to 10px.
- `border`, which sets the gap (in px) mayland leaves around the tiling space, between the windows and monitor edges. defaults to 20px.

### env

you can set environment variables inside mayland inside the `env` category.
you can unset variables by setting the to the default string.

```ini
env {
    QT_QPA_BACKEND = "wayland"
    SDL_VIDEODRIVER = "wayland"

    # you can unset variables like this
    DISPLAY = ""
}
```

### bind

you can configure keybindings in the `bind` category.

```ini
bind {
    mod+shift+escape = "quit"

    mod+q = "close"
    mod+v = "toggle-floating"

	mod+tab = "cycle" [ "next" ]
	mod+shift+tab = "cycle" [ "prev" ]

    mod+t = "spawn" [ "kitty" ]
    mod+e = "spawn" [ "nautilus" ]
    mod+n = "spawn" [ "firefox" ]
    mod+space = "spawn" [ "fuzzel" ]

    mod+1 = "workspace" [ 0 ]
    mod+2 = "workspace" [ 1 ]
    mod+3 = "workspace" [ 2 ]
    mod+4 = "workspace" [ 3 ]
    mod+5 = "workspace" [ 4 ]
    mod+6 = "workspace" [ 5 ]
}
```

keybinds are in the form of `<keymapping> = "<action>"`.

actions may have additional parameters. these are passed through via brackets as [tagged enums](https://github.com/m4rch3n1ng/mayfig?tab=readme-ov-file#tagged-enums).

keymappings are in the form `"{ <modifier>+ } <key>"`.
supported modifiers are `"super"` (with an alias of `"meta"`), `"ctrl"` and `"alt"`.
there is a special modifier `"mod"`, which is an alias for `"super"` when mayland is running standalone,
and an alias for `"alt"`, when mayland is running windowed.
both the `<key>` and all `<modifier>` are case-insenstive. there is no support for modifier-only shortcuts.

currently supported actions are:
- `"quit"`: quit the compositor.
- `"close"`: close the active window.
- `"toggle-floating"`: toggle the active window's floating state
- `"cycle" [ <direction> ]`: cycles through the windows with the given `<direction>`.
the direction can be either `"next"` or `"prev"`.
- `"workspace" [ <index> ]`: switch to workspace with the index `<index>`.
- `"spawn" [ <cmd> <... args> ]`: spawns the `<cmd>` as a command, with the other parameters as arguments.

### windowrules

you can configure window rules in the `windowrules` category.

```ini
windowrules {
	app-id [ "org.gnome.Nautilus" ] {
		floating = true
		opacity = 0.8
	}

	match [ "firefox" "/.*Mozilla Firefox.*/v" ] {
		floating = true
		opacity = 0.8
	}
}
```

window rules follow the pattern of `<matcher> [ <parameters> ] { <windowrules> }`.
in order for the windowrules to be applied, a window needs to be matched with one of
the `<matcher>`s first.

currently supported matchers are:

- `app-id`, which maches on the first parameter, the `app id` (i.e. the `class` as x11 calls it).
- `title`, which matches on the window `title`.
- `match`, which is a combination of the `app-id` and `title`, needing both to match.

you can opt into regexes for all of the currently supported matchers, by using a `"/<regex>/"` syntax.  
regexes by default are full string matches, i.e. they add an implicit `$(?:<regex>)^` around the regex.
if you don't want that, then you will have to manually add the `.*?` to the beginning and end of you regex like `/.*?<regex>.*?/`.  
if you are in regex mode, you can specify additional flags after the last slash, like `"/<regex>/<flags>"`.
currently supported flags are `i` to enable case-insensitive matching and `v` to invert the match.
you can enable multiple flags at the same time.

the `<windowrules>` struct currently has the following options:

- `floating`, which sets if a window should be floating on initial mapping.
- `opacity`, which sets the window opacity. takes a float between 0 and 1.
