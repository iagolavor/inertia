# Video debug panel (archived)

On-screen video pipeline debugger for Cursor Simple Browser (no DevTools). **Removed from the app** to keep the PR lean; restore from git if needed.

## Restore from git

Last shipped in commit `a26dee1` on branch `feature/video-p2p-chunks`:

```bash
git show a26dee1:apps/web/src/lib/video-debug.svelte.ts > apps/web/src/lib/video-debug.svelte.ts
git show a26dee1:apps/web/src/lib/components/VideoDebugPanel.svelte > apps/web/src/lib/components/VideoDebugPanel.svelte
```

Then re-wire:

1. Import debug helpers in `apps/web/src/lib/video.ts` from `$lib/video-debug.svelte` (see that commit).
2. Add `<VideoDebugPanel />` to `apps/web/src/routes/+layout.svelte`.

## What it did

- Fixed bottom-right panel listing pipeline steps with timestamps
- Mirrored `videoDebug()` logs (load, seek, capture, fileToBase64)
- Attached listeners for all HTMLMediaElement events on off-screen `<video>`
- Enabled in Vite dev automatically; in prod via `localStorage.setItem('inertia:video:debug', '1')`

## Files (removed)

| Path | Role |
|------|------|
| `apps/web/src/lib/video-debug.svelte.ts` | Log buffer + enable flag |
| `apps/web/src/lib/components/VideoDebugPanel.svelte` | UI panel |
| hooks in `video.ts` | `videoDebug`, `attachVideoDebugProbe`, `videoSnapshot` |
