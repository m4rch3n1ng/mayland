
### mayland-config

mayland uses the [mayfig](https://github.com/m4rch3n1ng/mayfig) language for configuration. you can take a look there
to find an explanation of the syntax.  
an example config can be found at [mayland.mf](https://github.com/m4rch3n1ng/mayland/tree/main/resources/mayland.mf).

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
        # xkb_file = "~/.config/keymap/may.xkb"

        # xkb_layout = "de,us"
        # xkb_model = "thinkpad"
        # xkb_variant = "nodeadkeys"
        # xkb_options = "ctrl:nocaps"

        repeat_delay = 600
        repeat_rate = 25
    }

    touchpad {
        tap = true
        tap_and_drag = false
        tap_drag_lock = false

        dwt = true
        dwtp = true

        natural_scroll = true
        # scroll_method = "two_finger"

        # click_method = "clickfinger"

        middle_emulation = true
        tap_button_map = "left_right_middle"
        left_handed = false

        accel_speed = 0
        # accel_profile = "adaptive"
    }

    mouse {
        natural_scroll = false

        middle_emulation = false
        left_handed = false

        accel_speed = 0
        # accel_profile = "adaptive"
    }
}
```

> [!TIP]
>
> you can also set device-specific configurations for touchpads and mice
>
> ```ini
> input {
>     mouse [ "<some gaming mouse>" ] {
>         accel_profile = "flat"
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
        xkb_layout = "de,us"
        xkb_model = "thinkpad"
        xkb_variant = "nodeadkeys"
        xkb_options = "ctrl:nocaps"
    }
}
```

alternatively you can set a path directly to a .xkb file containing an xkb keymap. this overrides all other xkb settings.

```ini
input {
    keyboard {
        xkb_file = "~/.config/keymap/may.xkb"
    }
}
```

##### repeat

you can change the key repeat config.

`repeat_delay` changes the delay in ms before the key starts repeating
`repeat_rate` changes the rate of repeated key preses per second

```ini
input {
    keyboard {
        repeat_delay = 600
        repeat_rate = 25
    }
}
```

#### pointing devices

the pointing devices `touchpad` and `mouse` share some of their config.

these include:

- `natural_scroll`, which inverts the scrolling direction if set to true. defaults to `true` for touchpads and `false` for mice.
- `accel_speed` controls the linear acceleration speed. accepts values between `-1.0` and `1.0`, and defaults to `0.0`.
- `accel_profile` controls the [pointer acceleration profile](https://wayland.freedesktop.org/libinput/doc/latest/pointer-acceleration.html#pointer-acceleration-profiles). can be set to `"adaptive"` (the default) and `"flat"` (disables pointer acceleration).
- `middle_emulation`. emulate a middle mouse click by pressing left and right at the same time. defaults to `true` for touchpads and `false` for mice.
- `left_handed` enables left-handed mode, which inverts left and right click. off by default.

settings specific to touchpads are:

- `tap` enables / disables tapping support. on by default.
- `tap_and_drag` enables the libinput [tap-and-drag](https://wayland.freedesktop.org/libinput/doc/latest/tapping.html#tap-and-drag) setting.
- `tap_drag_lock` controls the drag lock for `tap_and_drag`. both of these are off by default.
- `dwt` disables the touchpad while typing. defaults to `true`.
- `dwtp` disables the touchpad while trackpointing. defaults to `true`.

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

the format is `"<width>x<height>"` and allows you to optionally pass the refresh rate as "`<width>x<height>@<refresh-rate in hz>`".
if the refresh rate is omitted, mayland will pick the mode with the highest refresh rate,
otherwise it will pick the refresh rate that matches the given one as closely as possible.

if the mode is not set at all or the resolution does not exist, mayland will pick the preferred mode with the highest quality.

running `mayctl outputs` inside of mayland will list all outputs and all their available modes.

### cursor

you can configure your cursor theme and size in the `cursor` category.
mayland currently only accepts cursors in the xcursor format.

```ini
cursor {
    xcursor_theme = "Bibata-Modern-Classic"
    xcursor_size = 24
}
```

if `xcursor_theme` or `xcursor_size` is not set, mayland will attempt to read their values from the `XCURSOR_THEME` and `XCURSOR_SIZE`
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
    mod+shift+escape = "exit"
    mod+q = "close"
}
```

keybinds are in the form of `<keymapping> = "<action>"`.

actions may have additional parameters. these are passed through via brackets as tagged values. etc.

keymappings are in the form `"{ <modifier>+ } <key>"`.
supported modifiers are `"super"` (with an alias of `"meta"`), `"ctrl"` and `"alt"`.
there is a special modifier `"mod"`, which is an alias for `"super"` when mayland is running standalone,
and an alias for `"alt"`, when mayland is running windowed.
both the `<key>` and all `<modifier>` are case-insenstive. there is no support for modifier-only shortcuts.

currently supported actions are:
- `"quit"`: quit the compositor.
- `"close"`: close the active window.
- `"toggle_floating"`:
- `"workspace" [ <index> ]`: switch to workspace with the index `<index>`.
- `"spawn" [ <... args> ]`

### windowrules

you can configure window rules in the `windowrules` category.

```ini
windowrules {
	app_id [ "org.gnome.Nautilus" ] {
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

- `app_id`, which maches on the first parameter, the `app id` (i.e. the `class` as x11 calls it).
- `title`, which matches on the window `title`.
- `match`, which is a combination of the `app_id` and `title`, needing both to match.

you can opt into regexes for all of the currently supported matchers, by using a `"/<regex>/"` syntax.  
regexes by default are full string matches, i.e. they add an implicit `$(?:<regex>)^` around the regex.
if you don't want that, then you will have to manually add the `.*?` to the beginning and end of you regex like `/.*?<regex>.*?/`.  
if you are in regex mode, you can specify additional flags after the last slash, like `"/<regex>/<flags>"`.
currently supported flags are `i` to enable case-insensitive matching and `v` to invert the match.
you can enable multiple flags at the same time. 

the `<windowrules>` struct currently has the following options:

- `floating`, which sets if a window should be floating on initial mapping.
- `opacity`, which sets the window opacity. takes a float between 0 and 1.
