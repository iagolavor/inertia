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
- Header: avatar, name, bio.
- **Posts | Files** tabs on own and friend profiles. Each tab has a short how-it-works blurb, then a bordered panel with a sticky title/toolbar row (e.g. Add photo / New folder) and content below.
- **Posts**: durable photo grid on the author's device. Publishing a photo also emits a 7-day feed announcement.
- Friend profiles load live over P2P when the friend is online (thumbs auto-fetch on visit).
- **Files**: folder icons for author-hosted shared folders. Owner creates folders and adds files (zip drop or folder auto-zip). Friends browse and download only. Neutral naming (not a media catalog). Large peer downloads require a direct connection; see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md).
- Files UI is a small **finder-style** pane: sticky breadcrumb (`Files / folder`) and toolbar actions stay fixed; the content area below lists folders or files and accepts drops when a folder is open.

### Post
- Optional text + optional photo.
- Author, relative time, time until expiry (7d) or "saved" when archived.
- Simple card layout, no engagement chrome.

### Feed (home)
- Chronological, friends only (P2P contacts).
- User publishes → post stored locally → sent to contacts when peers are online.
- Ephemeral by default: disappears after 7 days unless local history is enabled.
- Separate from the durable profile grid.

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
| Profile | Durable photo grid (Posts tab) | `profile_items` + local blobs; friend view via `ProfileManifest` P2P |
| Files | Folder icons; opt-in download | `archive_folders` / `archive_entries` + chunked ingest; peer pull is DCUtR-only ([ARCHIVE-P2P.md](./ARCHIVE-P2P.md)) |
| Post | Card in feed | `ContentType::Post`, 7d TTL |
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
