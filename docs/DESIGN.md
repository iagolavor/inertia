# Inertia — Design Philosophy

## Essence

Inertia is a **local-first**, **ephemeral** social network for your inner circle. The visual language should feel **clean, minimal, and familiar** — in the basic sense of Instagram: photos front and center, a personal profile, a chronological feed — but **without** algorithms, ads, or doomscrolling.

---

## Visual principles

### 1. Less is more
- Generous whitespace (or dark surfaces in dark mode).
- One primary action per screen.
- Simple typography, no ornament.
- Soft corners (8–12px), minimal or no shadows.

### 2. Content first
- Photos and posts are the focus.
- Metadata (time remaining, delivery state) stays subtle.
- Avatars identify people; identicons are a fallback, not the hero.

### 3. Familiar, not copied
- Photo grid on the profile (visual reference, not a clone).
- Chronological feed of friends' posts.
- No stories, reels, public likes, or follower counts.

### 4. Honesty about state
- **Online / offline** indicator always visible next to the status dot.
- Failed deliveries visible in the outbox — transparency, not hiding errors.
- Ephemeral content: show when a post expires (or “saved” when archived locally).

---

## Palette and theme

| Token | Use |
|-------|-----|
| `--bg` | Main background |
| `--surface` | Cards, nav, panels |
| `--text` | Primary text |
| `--muted` | Captions, metadata, status labels |
| `--accent` | Links and primary actions |
| `--success` | Online, delivered |
| `--danger` | Offline, failure |

**Dark** and **light** mode supported. The user chooses; the system does not impose.

---

## Key components

### Status (online / offline)
Colored dot + textual label `online` or `offline` side by side. No ambiguity.

### Profile
- Header: avatar, name.
- **Personal photos**: local grid, stored on device.
- Posts live on the **Feed** tab, not on the profile screen.

### Post
- Optional text + optional photo.
- Author, relative time, time until expiry (48h) or “saved” when archived.
- Simple card layout, no engagement chrome.

### Feed (home)
- Chronological, friends only (P2P contacts).
- User publishes → post stored locally → sent to contacts when peers are online.
- Ephemeral by default: disappears after 48 hours unless local history is enabled.

### Settings
- Theme, optional feed history, backup export/restore, cryptographic identity details.

---

## What to avoid

- Infinite feeds optimized for retention.
- Aggressive notifications or “new activity” badges.
- Dense UI with too many buttons and tabs.
- Gradients, glassmorphism, or passing visual trends.
- Anything that suggests mass scale (followers, virality).

---

## Voice and tone (UI copy)

- Direct and calm.
- English by default for an international audience; avoid technical jargon on the surface.
- Explain P2P and ephemerality only when needed (onboarding, errors).

---

## Relationship to technical vision

This document complements [VISION.md](./VISION.md):

| Concept | Design | Technical |
|---------|--------|-----------|
| Profile | Photo grid | SQLite + local blobs |
| Post | Card in feed | `ContentType::Post`, 48h TTL |
| Feed | Chronological home | `local_posts` + friend inbox + optional `feed_archive` |
| Friends | Closed circle | P2P contacts |

---

## Layout reference

```
┌──────────────────────────────────────────┐
│ Inertia  [Feed|Profile|Settings]  ● online│
├──────────────────────────────────────────┤
│ Feed                                     │
│ ┌────────────────────────────────────┐   │
│ │ @name · 2h ago · 46h left          │   │
│ │ [optional photo]                   │   │
│ │ post text                          │   │
│ └────────────────────────────────────┘   │
│ ...                                      │
├──────────────────────────────────────────┤
│ Profile                                  │
│ [avatar] Name                            │
│ ┌───┬───┬───┐                            │
│ │ + │ 📷│ 📷│  photos                    │
│ └───┴───┴───┘                            │
├──────────────────────────────────────────┤
│ Settings                                 │
│ Theme · Feed history · Backup · Keys     │
└──────────────────────────────────────────┘
```
