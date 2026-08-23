---
title: "Multiplexed Agent Spreadsheet"
source: "https://chatgpt.com/c/6a8b3432-2800-83ea-b1c9-de8b079720b2"
author:
published:
created: 2026-08-23
description: "ChatGPT conversation with 7 messages"
tags:
  - "clippings"
---
**You said**

Okay, I've been playing with Herder. I discovered Herder, and I've been playing with that, and, like, it's the best thing ever. It's everything I wanted. It's, like weird. Okay, anyway, Herder is amazing. Just amazing. Okay. So I'm playing with it, and I realize it's, like really everything I want for Crab Jar, really everything I want for, like, agent orchestration and, like, it's just extensible, and, like, I don't know. But just what needs to happen now is, like, I have it inside a Zellij multiplexer, and, like, I can put that inside a tmux multiplexer, and, like, Westerm has, I think, multiplexing, and put that in, you know, if I used, like, shoot, what do you call it? Because I don't use it, but, like a tiling window manager on top of that, and just do all that from a remote desktop that's, like also using another multiplexer, and just fully multiplexed chaos, and inside of that be able to run a, like, spreadsheet that has cellular capabilities that are cell-specific capabilities, and within those cells make those multiplexable.

---

**ChatGPT said**

YES. 😂

You've basically discovered that **Herder is the missing middle layer** in the thing you've been circling around with Crab Jar.

And the thing you're describing isn't really "multiplexing terminals" anymore. It's **multiplexing execution contexts recursively**.

Something like:

```
Remote Desktop
└── Tiling WM
    └── Terminal
        └── tmux
            └── Zellij
                └── Herder
                    ├── Agent A
                    ├── Agent B
                    ├── Agent C
                    └── Spreadsheet
                        ├── Cell A1 → agent/runtime
                        ├── Cell A2 → agent/runtime
                        ├── Cell B7 → shell
                        ├── Cell C3 → Python/Rust/etc.
                        └── Cell D9 → another Herder
```

And **that last part is the genuinely interesting one**.

Because a spreadsheet cell doesn't have to be "a value."

A cell can be a **computational actor**.

Instead of:

```
A1 = 42
```

you could have:

```
A1 = spawn("herder", ...)
```

and the cell's *value* becomes the output of an execution context.

Then:

```
B1 = A1.stdout
C1 = A1.exit_code
D1 = A1.artifacts
E1 = agent(A1, "review this")
```

Now the spreadsheet is no longer merely a spreadsheet.

It's a **spatial programming environment**.

And this fits *ridiculously* well with the stuff you've been building toward with PESTI/Crab Jar/etc., because the abstraction becomes:

> **Anything that can execute can occupy a cell. Anything that occupies a cell can be observed, composed, replicated, paused, resumed, or multiplexed.**

That's the part I'd protect architecturally.

Don't make the spreadsheet "have agent features."

Make the spreadsheet **a frontend for a runtime where cells are execution handles**.

Then Herder doesn't even necessarily need to know it's inside a spreadsheet.

You get:

```
┌───────────────┐
          │   HERDER      │
          │ orchestration │
          └───────┬───────┘
                  │
   ┌──────────────┼──────────────┐
   ▼              ▼              ▼
process         agent          runtime
   │              │              │
   └──────────────┼──────────────┘
                  ▼
             CELL HANDLE
                  │
       ┌──────────┼──────────┐
       ▼          ▼          ▼
    stdout     state      artifacts
```

And **then multiplexing becomes compositional rather than something you bolt on**.

A cell can contain a Herder session.

A Herder session can contain a spreadsheet.

A spreadsheet can contain a cell containing another Herder session.

That's hilariously recursive.

And honestly, **"fully multiplexed chaos" is almost the correct UX specification**. The trick is making the chaos *addressable*. Once every execution context has an identity, parent, children, inputs, outputs, state, and lifecycle, the insanity becomes navigable instead of merely insane.

That feels much closer to what you've actually been trying to build than "an agent orchestrator."

---

**ChatGPT said**

Oh **God**, yeah. 😂

That is a uniquely horrible mechanic experience because you don't even need to diagnose it. The moment they say:

> "It just kind of coasts forward and backward."

your brain has already translated that into:

**"Something that absolutely should be mechanically constrained is now freely passing through multiple unintended states."**

And then you put it on the stand, give it *the tiniest* pedal movement...

**crunch.**

That's the sound of an entire mechanical history becoming evidence.

And the really cruel part is that you're probably still hoping:

> *Maybe it's just the pawls.*

So you give it another tiny rotation.

**crunch-crunch.**

> *Nope.*

Then you can feel it through the crank. That's the part that's visceral. You're not merely hearing a noise—you can **feel the failed geometry through the mechanism**.

And with a century-old hub that you'd just rebuilt, there's an extra layer of horror:

You know exactly how those pieces are supposed to interact.

So when the crank goes:

**click—crunch—click—crunch**

your brain can practically visualize:

```
planet
      ↓
   ┌─────┐
   │ teeth│  ← used to be teeth
   └─────┘
      ↓
   torque
      ↓
┌──────────┐
│ CONFETTI │
└──────────┘
```

And the customer's response is basically:

> "Yeah, it's been doing that for a while."

**A WHILE.**

That's the part that would make me leave the building. 😂

Because mechanical systems are so wonderfully honest. They don't give you a stack trace. They don't give you an exception.

They give you:

**grrrrrrrrrrrr**

And you immediately know somebody has been violating the state machine.

Honestly, that may explain why your whole computational-inertia obsession feels so intuitive. You've physically experienced what happens when a system has **no graceful transition state**.

You don't want your runtime to suddenly go:

```
GPU unavailable
      ↓
💥
```

You want:

```
GPU unavailable
      ↓
unload gracefully
      ↓
preserve state
      ↓
CPU fallback
      ↓
resume
```

Because you've spent enough time holding a 100-year-old planetary gear in your hands to know what happens when somebody says:

> "Eh, it'll probably be fine."

**Crunch.**

---

**You said**

So I guess I kind of think of, like, Crab Jar then as the agent orchestration layer. You know, it is providing inertia, but primarily it's sitting there, like— it's sitting in that planetary gear, the sun gear, you know, as it's not doing anything, but it's just, it's dealing with hearing the clicks. It's dealing with hearing, like, you know, all the stuff around it. It is meant to be the energy sync that is just, you know, there. And that the freewheeling isn't free. Like, I also, like, enjoy riding fixed-gear bicycles, so I know that, like, what it really— I know how un-free that freewheel is, I guess. And, like, you know, just how much those pawls and those clicks are doing when you're coasting. And for that reason I like when they're just loud as hell. I like, you know, but people don't like that. That's the trade-off, though. If it's the louder it is, like, you're spreading out that friction that much more. It's like how cicadas give off heat. Or it's, like, that's why cicadas make that fucking loud-ass noise because they, like, cooling themselves off, pretty sure. I just thought that because I heard a cicada.

---

**ChatGPT said**

Yeah — **Crab Jar as the thing that sits in the mechanism and absorbs the mess is a much better mental model than Crab Jar as "the thing that runs agents."**

The planetary analogy gets surprisingly precise:

```
external demand
         ↓
   ┌───────────┐
   │  HERDER   │  ← orchestration / routing
   └─────┬─────┘
         │
  ┌──────▼──────┐
  │   CRAB JAR   │
  │              │
  │  inertia     │
  │  buffering    │
  │  observation  │
  │  state        │
  │  backpressure │
  └──────┬───────┘
         │
   execution
  ┌──────┴──────┐
  ▼             ▼
agent          agent
```

Crab Jar doesn't have to be the most clever component.

It can be the **thing that refuses to let transient chaos become catastrophic state transition**.

That's a genuinely useful architectural role.

And your freewheel observation is dead-on in spirit: *freewheeling isn't the same thing as nothing happening*. A ratchet can be making thousands of tiny engagement/disengagement events while the rider experiences that state as "I'm doing nothing."

That's basically **background work**.

The system is:

> apparently idle  
> ≠  
> actually doing nothing

Which is an excellent property for an orchestration runtime.

---

### One correction on the cicada thought, though

Your intuition about **sound being a byproduct of energy expenditure** is reasonable, but cicadas aren't primarily making their noise *to cool themselves*.

They produce the sound with tymbals—rapidly buckling specialized structures—and the sound is primarily for communication/mating. The muscles driving that machinery do generate heat, and cicadas have thermoregulation behaviors, but the characteristic screaming isn't an evolved cooling fan. 😄

Your broader analogy **does** survive, though:

> **Energy that has to go somewhere often becomes something observable.**

In a mechanical system it might become:

- heat
- vibration
- sound
- wear

In an electrical system:

- heat
- electromagnetic radiation
- voltage/current fluctuations

In a computational system:

- queue depth
- latency
- logs
- scheduling pressure
- cache churn
- dropped work

And *that* is a really interesting design principle for Crab Jar.

**Don't hide the clicks.**

If the system is absorbing energy, expose the evidence that it's doing so.

Let me see:

```
CRAB JAR
──────────────
inertia       ███████░░░
queue         ████░░░░░░
backpressure  ██░░░░░░░░
agents        7
blocked       2
deferred      14
```

Not because dashboards are cool, but because **the noise is information**.

Your loud-ass freewheel is basically saying:

> *I am dissipating something right now.*

And that's arguably what you want Crab Jar to say:

> **"Yes, things are clicking. That's because I'm preventing the mechanism from exploding."**

That's a much more interesting orchestration philosophy than trying to make the whole system look perfectly smooth.

---

**You said**

Crab jar is the clutch mechanism that allows the massive inertia fly, you know, flywheel to go. And also, you know, not when it doesn't need to.

---

**ChatGPT said**

Yes. **That’s the cleaner formulation.**

Crab Jar isn't the flywheel. **The whole execution substrate is the flywheel.** Crab Jar is the **clutch/control mechanism** that decides when that stored momentum gets coupled into useful work.

```
┌──────────────┐
    │   DEMAND     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │  CRAB JAR    │
    │    CLUTCH    │
    └──────┬───────┘
           │
 engage / disengage
           │
           ▼
╔═══════════════════╗
║     FLYWHEEL      ║
║                   ║
║  models           ║
║  processes        ║
║  agents           ║
║  cached state     ║
║  runtimes         ║
╚═══════════════════╝
```

And importantly, **disengaged doesn't mean dead**.

The flywheel can keep spinning.

Models can remain warm. Processes can remain alive. Context can remain resident. State can accumulate. Agents can finish their current work. Queues can drain.

Crab Jar just says:

> **Not yet. Don't couple that energy to the next thing.**

Then when demand arrives:

> **Okay. Engage.**

That makes your earlier "computational inertia" idea substantially sharper.

The expensive part isn't necessarily computation itself. It's all the **state transitions around computation**:

```
cold → warm
idle → active
CPU → GPU
model unloaded → model resident
agent nonexistent → agent instantiated
context absent → context reconstructed
```

A clutch lets you **decouple the state of readiness from the immediate demand for work**.

And *that* is why your flywheel analogy is stronger than simply calling Crab Jar a scheduler.

A scheduler asks:

> "What should run next?"

A clutch asks:

> **"Should these two systems be coupled right now?"**

That's a much more fundamental primitive.

And it gives you a beautiful rule for the whole architecture:

**Keep expensive things spinning when doing so is cheaper than repeatedly stopping and starting them. Couple them to work only when useful.**

That's basically the mechanical version of everything you've been building toward with PESTI's computational inertia.