---
title: "ASCII Crab Animation"
source: "https://chatgpt.com/c/6a6556ef-2494-83ea-8d0d-a00ca899be9c"
author:
published:
created: 2026-07-26
description: "ChatGPT conversation with 5 messages"
tags:
  - "clippings"
---
**ChatGPT said**

Absolutely. I actually think **Crabjar** has an opportunity to have a recognizable little mascot instead of just a logo. Since it's a Ratatui application, I'd lean into **Unicode/Braille/Block glyph animation** rather than traditional ASCII.

Here are a few ideas.

## 1\. Braille "walking" crab (my favorite)

Braille characters are 2×4 pixels packed into one glyph, so they animate beautifully.

Example idle cycle:

```
⢀⣀⡀
⣾⣿⣷
⠘⠿⠃

⢀⣀⡀
⣾⣷⣿
⠈⠿⠁

⢀⣀⡀
⣿⣾⣷
⠐⠿⠂
```

Very low bandwidth, surprisingly expressive.

---

## 2\. Nerd Font crab

If you're already assuming developers use Nerd Fonts:

```
(\_/)
<(o   o)>
 /  ^  \
/|_| |_|
```

or

```
\/     \/
( ᗜ )
/|___|\
```

Animate only the claws.

---

## 3\. Tiny ASCII

```
(\ /)
( . .)
/ V \
```

Animation:

```
(\ /)
( . .)
/ V \

(/ \)
( . .)
\ V /
```

---

## 4\. Unicode Box Drawing

```
╭──╮
├██┤
╰┬┬╯
```

Then wiggle the legs.

---

## 5\. Block Pixel Crab

```
▄██▄
████
▀▀▀▀
```

Very "retro terminal."

---

## Spinner style

Instead of a spinner...

```
🦀
```

becomes

```
<:>
<]>
<}>
<)>
```

or

```
ᗧ
ᗤ
ᗣ
ᗢ
```

Looks like the crab is scuttling sideways.

---

## Ratatui Animation

Ratatui already redraws every frame, so animation is simply:

```rust
const FRAMES: &[&str] = &[
    r#"
 (\_/)
( •• )
/|_|_\
"#,
    r#"
 (\_/)
( •• )
\|_|_/
"#,
];

let frame = FRAMES[tick % FRAMES.len()];
```

The Ratatui rendering model is designed around updating widgets each draw cycle, making this kind of frame animation straightforward. [^1]

---

## If you want to get fancy...

There are a couple of crates worth looking at:

- **`artbox`** — renders FIGlet text, sprites, and integrates directly with Ratatui, making it useful for logos and sprite-like terminal art. [^2]
- **`cascii`** — converts GIFs/videos into ASCII frame sequences, so you could prototype an animation in Aseprite or Krita and export terminal frames. [^3]
- **`zoa`** — if you ever wanted an absurdly over-the-top spinning 3D crab in your terminal. It embeds directly as a Ratatui widget. [^4]

---

## What I'd build for Crabjar

Knowing the aesthetic you've been gravitating toward (Rust ecosystem, Zellij, Helix, hacker-ish, 8-bit), I'd skip "cute mascot" art and instead make Crabjar feel like a classic Unix daemon.

Something like this:

```
__
  ___(o )>
  \ <_. )
   \`---'

crabjar
```

and during work:

```
__
___(o )>
\ <_. )    indexing...
 \`---'
```

with just the claw and legs alternating every 100–150 ms so it appears to scuttle while processing. It's subtle, readable over SSH, and feels like it belongs in a terminal rather than being a transplanted logo. I think that would fit the rest of your Rust-first tooling nicely.

---

**You said**

yes! great call!

---

**ChatGPT said**

Haha, I'm glad that landed. I think there's an opportunity to make **Crabjar instantly recognizable** without ever needing a graphical logo.

I'd lean into the aesthetic of old Unix daemons and BBS software. Something that looks like it belongs in `htop`, `btop`, `git`, or `lazygit`.

## Idle

```
__
___(o )>
\ <_. )
 \`---'
```

## Walking

Frame 1

```
__
___(o )>
\ <_. )
 \`---'
```

Frame 2

```
__
___(o )>
 \<_. )
  \`---'
```

Frame 3

```
__
___(o )>
/ <_. )
 \`---'
```

Just moving the legs a tiny amount is enough for your brain to perceive motion.

---

## Thinking

While embedding or indexing:

```
__
___(- )>
\ <_. )
 \`---'
```

↓

```
__
___(o )>
\ <_. )
 \`---'
```

Blink every couple of seconds.

---

## Happy

```
__
___(^ )>
\ <_. )
 \`---'
```

---

## Error

```
xx
___(x )>
\ <_. )
 \`---'
```

or maybe the shell cracks 😄

```
__
___(x )>
\ </. )
 \`---'
```

---

## Loading animation

Instead of a spinner...

```
⠁
⠂
⠄
⠂
```

have Crabjar slowly crawl across the status bar.

```
[>                     ]

 __
(o )>
```

↓

```
[=======>              ]

      __
  ___(o )>
```

↓

```
[===============>      ]

             __
         ___(o )>
```

It gives the impression that **the crab is physically carrying your data into the jar**.

---

## Startup

I would absolutely do something like:

```
__
  ___(o )>
  \ <_. )
   \`---'

Crabjar v0.3.0
────────────────────────────────────────
Loading plugins...
Opening journal...
Scanning clipboard...
Ready.
```

Very "1993 Unix utility."

---

## Then...the jar

Eventually, imagine this:

```
__
___(o )>      ______
\ <_. )      /      \
 \`---'      |        |
            |        |
             \______/
```

As captures happen...

```
__
___(o )>      ______
\ <_. ) ---> | #### |
 \`---'       | #### |
              \______/
```

The jar slowly fills over the course of the session.

When indexing:

```
__
___(o )> =>=>=> ______
\ <_. )       | #### |
 \`---'        | #### |
               \______/
```

---

## Even better...

Imagine Crabjar never has a progress bar.

The crab **is** the progress bar.

```
Searching...

__                                  📦
(o )>------------------------------->
```

or

```
Embedding...

__      📄      📄      📄
(o )>  ->      ->      ->
```

The little crab becomes the visual language of the application. Users won't think "there's a loading spinner"—they'll think "the crab is working."

That kind of small, consistent detail is surprisingly memorable. Projects like `btop`, `lazygit`, `dust`, and `zellij` all have a distinctive terminal personality, and I think a tiny animated crab carrying "memories" into a jar would give Crabjar that same kind of identity.

---

**You said**

I made a toy project when I was reading TRPL: i can't find it right now but it was a little scurrying crab that looked like this >~{,,∞,}~<

---

**ChatGPT said**

😂 That's fantastic.

```
>~{,,∞,}~<
```

It has exactly the kind of weird terminal personality that sticks in your head. My first thought wasn't even "crab"—it was "some tiny creature scurrying across the screen," which is almost better.

I can see animating it with almost no changes.

### Scurry

```
>~{,,∞,}~<
 >~{,,∞,}~<
  >~{,,∞,}~<
   >~{,,∞,}~<
```

Or alternate the legs:

```
>~{,,∞,}~<
>≈{,,∞,}≈<
>~{,,∞,}~<
>≈{,,∞,}≈<
```

Or wiggle the claws:

```
>~{,,∞,}~<
<~{,,∞,}~>
>~{,,∞,}~<
<~{,,∞,}~>
```

Even just nudging the body one character every frame makes terminals feel surprisingly alive.

---

## If you embrace Unicode

You can get a little more expressive:

```
>╲{◉◉∞}╱<
```

or

```
>⟨••∞••⟩<
```

or

```
>~{··🦀··}~<
```

although I honestly like your original more because it's entirely text.

---

## One thing I'd absolutely steal

Since the project is **Crabjar**, what if the crab occasionally drags things?

```
>~{,,∞,}~<   📄
```

then

```
>~{,,∞,}~<──📄
```

then

```
______
>~{,,∞,}~< | 📄  |
           |____|
```

or with pure ASCII:

```
>~{,,∞,}~< --> [note]
```

Eventually:

```
_________
     /         \
    |  notes   |
    |  shell   |
    |  git     |
     \_________/

>~{,,∞,}~<
```

---

## This also feels very Rust

Rust projects tend to have a bit of personality without being flashy. Think about `cargo`, `ripgrep`, `bat`, `dust`, `zellij` —they're serious tools, but they often include tiny touches that make them feel handcrafted.

A tiny ASCII crab that occasionally wanders across the bottom of the UI carrying a document into the jar fits that tradition perfectly.

I would even make it configurable:

```toml
[mascot]
enabled = true
animation = "scurry"
speed = 120
wander = true
carry_items = true
```

Then power users who want a perfectly static TUI can disable it, while everyone else gets a little companion quietly doing work in the background. It's the sort of detail people end up mentioning in screenshots: "Wait... does the crab actually carry files into the jar?"

[^1]: [ratatui.rs](https://ratatui.rs/?utm_source=chatgpt.com)

[^2]: [docs.rs](https://docs.rs/artbox/latest/artbox/?utm_source=chatgpt.com)

[^3]: [docs.rs](https://docs.rs/crate/cascii/0.17.0?utm_source=chatgpt.com)

[^4]: [docs.rs](https://docs.rs/zoa/latest/zoa/?utm_source=chatgpt.com)