# Inertia - Design Philosophy

## Essence

Inertia is a **decentralized**, **low-cost**, **local-first**, **ephemeral** social network. The visual language should feel **clean, minimal, and familiar**: photos front and center, a personal profile, and a chronological friends feed.

---

## Visual principles

### 1. Less is more
- Generous whitespace (or dark surfaces in dark mode).
- One primary action per screen.
- Simple typography with calm, readable type.
- Soft corners (8-12px) and light elevation when needed.

### 2. Content first
- Photos and posts are the focus.
- Metadata (time remaining, delivery state) stays subtle.
- Avatars identify people; identicons are a fallback when someone has not set a photo.

### 3. Familiar basics
- Photo grid on the profile.
- Chronological feed of friends' posts.
- Private messages and a clear connections surface for invites.

### 4. Honesty about state
- **Online / offline** indicator always visible next to the status dot.
- Failed deliveries visible in the outbox so senders can retry or wait for expiry.
- Ephemeral content: show when a post expires (or "saved" when archived locally).

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
Colored dot + textual label `online` or `offline` side by side so connection state is clear at a glance.

### Profile
- Header: avatar, name, bio.
- **Posts | Files** tabs on own and friend profiles. Each tab has a short how-it-works blurb, then a bordered panel with a sticky title/toolbar row (e.g. Add photo / New folder) and content below.
- **Posts**: durable photo grid on the author's device. Publishing a photo also emits a 7-day feed announcement.
- Friend profiles load live over P2P when the friend is online (thumbs auto-fetch on visit).
- **Files**: folder icons for author-hosted shared folders. Owner creates folders and adds files (zip drop or folder auto-zip). Friends browse and download. Neutral naming for everyday shared files. Large peer downloads require a direct connection; see [ARCHIVE-P2P.md](./ARCHIVE-P2P.md).
- Files UI is a small **finder-style** pane: sticky breadcrumb (`Files / folder`) and toolbar actions stay fixed; the content area below lists folders or files and accepts drops when a folder is open.

### Post
- Optional text + optional photo.
- Author, relative time, time until expiry (7d) or "saved" when archived.
- Simple card layout focused on the content itself.

### Feed (home)
- Chronological, friends only (P2P contacts).
- User publishes → post stored locally → sent to contacts when peers are online.
- Ephemeral by default: disappears after 7 days unless local history is enabled.
- Separate from the durable profile grid.

### Settings
- Theme, optional feed history, backup export/restore, cryptographic identity details.

---

## Design preferences

Prefer surfaces that stay calm and readable:

- Finite, chronological feeds that respect the 7-day lifecycle.
- Quiet notifications and badges that matter (for example unread messages), used sparingly.
- Few primary actions per screen; clear hierarchy over dense toolbars.
- Timeless layout and color tokens instead of short-lived visual trends.
- Language and chrome that fit a decentralized, local-first social product.

---

## Voice and tone (UI copy)

- Direct and calm.
- English by default for an international audience; keep jargon for settings and errors where it helps.
- Explain P2P and ephemerality when the user needs context (onboarding, connection issues, expiry).

---

## Relationship to technical vision

This document complements [VISION.md](./VISION.md):

| Concept | Design | Technical |
|---------|--------|-----------|
| Profile | Durable photo grid (Posts tab) | `profile_items` + local blobs; friend view via `ProfileManifest` P2P |
| Files | Folder icons; opt-in download | `archive_folders` / `archive_entries` + chunked ingest; peer pull is DCUtR-only ([ARCHIVE-P2P.md](./ARCHIVE-P2P.md)) |
| Post | Card in feed | `ContentType::Post`, 7d TTL |
| Feed | Chronological home | `local_posts` + friend inbox + optional `feed_archive` |
| Friends | Invite-based roster | P2P contacts |

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
