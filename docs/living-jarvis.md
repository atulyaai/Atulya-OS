# Living Jarvis — Experience Architecture (AtulyaOS)

> **Scope:** This document defines the *experience layer* that turns AtulyaOS from a
> conventional kernel into a **living, conversational presence** ("Jarvis"). The
> low-level mechanical architecture (GDT, IDT, VFS, SYSCALL, etc.) is covered in
> [`architecture.md`](./architecture.md) and is **assumed to exist**. This doc
> describes what must be *added* and how the pieces fit into one living system.
>
> **Status:** Blueprint. Mechanical scaffolding exists in-repo; the experience
> layer is currently prototyped only in `../atulya-preview` (host GIF).

---

## 0. What "Living" Means

A normal OS shows a desktop. Jarvis is a **presence** that is always aware and
always reachable. Five properties define "alive":

| Property | Meaning | Where it lives |
|----------|---------|----------------|
| **Persistent** | The face is always rendering, never a static screenshot | `LivingShell` (L2) |
| **Aware** | Reflects system state (CPU/mem/net, listeners) in real time | telemetry → HUD rings |
| **Conversational** | Wake-word + spoken dialogue, not menus | Voice + Cognition (L3–L4) |
| **Anticipatory** | Runs an agent that can act, not just answer | Cognition + Action (L4–L5) |
| **Reactive** | Visual/audio feedback to every event | HUD state machine + `sound.rs` |

The existing `boot/awakening.rs` already nails the *entrance*. The gap is that it
**stops** — the OS then drops into a placeholder `desktop.rs`. The whole point of
this doc is: **the boot animation must become the desktop, and the desktop must
stay alive and start listening.**

---

## 1. Layered Model

```
 L5  ACTION      ┌─ TTS (sound.rs) · skills · system control · apps ─┐
 L4  COGNITION   │  wake-word · ASR · LLM brain (ai_model/gguf) ·   │
 L3  PERCEPTION  │  voice capture · vision (camera) · input         │
 L2  PRESENCE    │  LivingShell: breathing core, rings, voice pill,  │
                 │  glass panels — ALWAYS RENDERING                  │
 L1  RUNTIME     │  memory · scheduler · timer · drivers (existing) │
 L0  BRING-UP    │  bootloader → kernel → framebuffer (BLOCKED, §7)  │
                 └──────────────────────────────────────────────────┘
                            every layer feeds the HUD
```

L0–L1 are mechanical (see `architecture.md`). **L2–L5 are the living Jarvis and
are specified here.**

---

## 2. L2 — The Living Shell (the "face")

### 2.1 Concept
A single, always-on render surface. Not windows; a **central intelligence core**
surrounded by living geometry. This is the visual identity prototyped in
`atulya-preview` and must be ported into the kernel.

### 2.2 Compositional elements (all in `kernel/src/shell/`)
- **Core** (`hud.rs`): a breathing central orb — layered `draw_glow_orb` pulses
  with `sin(t)`. The "heartbeat" of the system.
- **Particle field**: 48–64 ambient particles orbiting on integer-trig (`math.rs`
  `sinish`/`cosish`), radius breathing gently.
- **Telemetry rings**: 3 concentric arcs (cyan/magenta/amber) driven by live
  system metrics (CPU load, memory, net) via `progress_arc`.
- **HUD dials**: counter-rotating dotted/dashed orbital ticks (`draw_arc_ticks`).
- **Voice-wake indicator**: a pulsing pill `◌ SAY 'ATULYA'` that lights when
  listening.
- **Masthead**: `ATULYA OS` + `ONLINE` status, corner HUD brackets.
- **Glass panels**: rounded, alpha-blended info cards (system vitals, last
  utterance, active skill) using `display.rect_rounded_alpha`.

### 2.3 State machine
The shell is never "static." It is always in one of:

```
        ┌─────────┐   wake detected   ┌──────────┐  ASR done  ┌──────────┐
        │  IDLE   │ ───────────────▶ │ LISTENING│ ──────────▶ │ THINKING │
        │(breath) │ ◀─────────────── │(pulse)   │ ◀────────── │(spin)    │
        └─────────┘  timeout         └──────────┘  error     └──────────┘
             ▲                            │                      │
             │                      TTS start              response ready
             │                            ▼                      ▼
             └───────────────────────┌──────────┐ ◀────────┌──────────┐
                                     │ SPEAKING │          │  ALERT   │
                                     │(waveform)│          │(red ring)│
                                     └──────────┘          └──────────┘
```
Each state only changes *visual treatment* — the render loop never stops.

### 2.4 Render contract
Reuse `display.rs` exactly as-is (it is already the compositor). The shell writes
directly to the framebuffer every frame, paced by `timer::now_ms()` to a real
frame budget (the `awakening.rs` loop already does this correctly — the bug it
fixed was uncalibrated spin-delay).

---

## 3. L3–L4 — Perception & Cognition Pipeline

```
  microphone ─▶ voice.rs (capture/RingBuf)
        │
        ▼
  wake-word detector ("ATULYA")  ──(no)──▶ stay IDLE
        │ (yes)
        ▼
  ASR (speech→text)  ─▶ ai.rs (intent parse)
        │
        ▼
  LLM brain (ai_model.rs + gguf.rs)  ─▶ response text + optional action
        │
        ▼
  L2 state → THINKING → SPEAKING
        │
        ▼
  TTS (sound.rs) ─▶ speaker
        │
        ▼
  back to IDLE (listening for next wake)
```

### 3.1 Modules (map to existing files)
- `voice.rs` — PCM capture ring buffer + (TODO) wake-word + ASR hooks.
- `ai.rs` — intent graph; today does VFS search/dispatch, must grow into the
  orchestrator that calls the brain and selects skills.
- `ai_model.rs` / `gguf.rs` — on-device LLM inference. **Decision needed (§9):**
  on-device `gguf` vs. a small remote API. On-device is the "sovereign OS"
  promise; remote is faster to demo.
- `sound.rs` — PIT chime today; must add a TTS path (e.g., a bundled
  phoneme/sample synth or a streaming codec decoder).

### 3.2 Concurrency
The voice/cognition loop runs as a **kernel task** under `scheduler.rs`, not
blocking the shell render. Shell renders every frame; the agent task advances
asynchronously and only signals state changes to the HUD.

---

## 4. L5 — Action

- **TTS** (L4→L5): `sound.rs` becomes the speech output path.
- **Skills**: the `wasm/` runtime is the sanctioned skill sandbox; an intent can
  load+run a WASM skill.
- **System control**: `process.rs`, `syscall.rs`, `fs/` let the agent act on the
  machine (open viewer, manage files, launch tasks).
- **Vision** (`vision.rs`): camera frames feed both perception (who is present)
  and skills (OCR, scene understanding).

---

## 5. Boot → Living Desktop → Agent (the new flow)

Replace the flow in `architecture.md §2` tail:

```
  ... J[Boot Awakening]  ──▶  K[Biometric Login Gate]
       K  ──▶  L[LivingShell::start()]   // replaces "static desktop"
                 │
                 ├─ render loop NEVER ends (IDLE breathing)
                 ├─ spawn voice/cognition task (LISTENING-capable)
                 └─ first utterance possible immediately
```

Concretely:
- `boot/awakening.rs::run` plays once, then calls `shell::LivingShell::run`.
- `LivingShell` owns the particle/core state and the state machine.
- `main.rs` spawns the agent task *before* or during shell start.

---

## 6. Module Map (Jarvis stack)

| Layer | Module (proposed path) | Status | Notes |
|-------|------------------------|--------|-------|
| L2 | `kernel/src/shell/mod.rs` | TODO | owns state machine + loop |
| L2 | `kernel/src/shell/hud.rs` | TODO | core/rings/particles (ported from preview) |
| L2 | `kernel/src/shell/state.rs` | TODO | IDLE/LISTEN/THINK/SPEAK/ALERT |
| L2 | `kernel/src/shell/panels.rs` | TODO | glass info cards |
| L0→L2 | `boot/awakening.rs` | EXISTS | entrance; hand off to shell |
| L1 | `display.rs`, `font.rs`, `math.rs` | EXISTS | compositor/trig — reuse as-is |
| L3 | `voice.rs` | SCAFFOLD | add wake-word + ASR |
| L4 | `ai.rs` | SCAFFOLD | grow into orchestrator |
| L4 | `ai_model.rs`, `gguf.rs` | SCAFFOLD | LLM brain |
| L4 | `sound.rs` | SCAFFOLD | add TTS path |
| L5 | `wasm/runtime.rs` | EXISTS | skill sandbox |
| L5 | `vision.rs` | SCAFFOLD | camera perception |

---

## 7. Build / Toolchain Gate (must be solved first)

**The OS does not currently build.** `Cargo.toml` + `build.rs` use
`bootloader 0.11.x`, whose transitive `x86_64 0.15.2` has a `Step` trait impl
incompatible with **every** modern Rust (floating nightly requires a method the
2026-06-02 toolchain rejects). No nightly fixes it.

**Required:** replace the boot mechanism (recommend **limine** — modern,
compiles cleanly, BIOS+UEFI, simple config). This means:
1. Rewrite kernel entry from `bootloader_api` to limine's boot-info protocol.
2. Replace `build.rs` disk-image step with limine's `limine-deploy` + ISO/IMG.
3. Update `scripts/run-qemu.ps1`.

Until §7 is done, the *only* runnable surface is the host preview
(`../atulya-preview`), which already validates L2 visuals end-to-end.

---

## 8. Phased Roadmap (ties to `ROADMAP.md`)

1. **Unblock boot** (limine) — gate for everything real.
2. **LivingShell L2** — port preview HUD into kernel; boot hands off; always-on.
3. **Voice loop L3–L4** — wake-word + TTS + LLM wired (host-prototype first).
4. **Agent actions L5** — skills + system control via intents.
5. **Polish** — persona, sound design, vision.

---

## 9. Open Decisions (need your call)

1. **LLM**: on-device `gguf` (sovereign, slow) vs. small remote API (fast demo)?
2. **Wake-word / ASR**: bundle a tiny engine (e.g., whisper.cpp port) or assume
   a streaming service?
3. **TTS**: sample/phoneme synth vs. codec playback vs. remote?
4. **Persona**: name is "Atulya"; voice gender/tone/ID? ("Jarvis" is the vibe,
   "Atulya" is the name — confirm).
5. **Visual identity**: palette (cyan/magenta/amber) + motion from the preview —
   accept as canonical, or iterate?

---

## 10. See It Now (no boot required)

`../atulya-preview` renders the **exact L2 experience** (boot → living desktop)
to an animated GIF using the real `display.rs`/`font.rs`/`math.rs`/`awakening.rs`
logic ported 1:1. Run `cargo run` there to produce `atulya-jarvis.gif`. This is
the visual contract the kernel port must match.
