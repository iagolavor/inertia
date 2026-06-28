/** Peer id suffix from a full relay multiaddr, if present. */
export function relayPeerId(multiaddr: string): string | null {
  const match = multiaddr.trim().match(/\/p2p\/([^/]+)$/);
  return match?.[1] ?? null;
}

/** True if `candidate` duplicates an existing relay (same peer id or exact string). */
export function isDuplicateRelay(candidate: string, existing: string[]): boolean {
  const trimmed = candidate.trim();
  if (!trimmed) return true;
  if (existing.some((addr) => addr.trim() === trimmed)) return true;
  const peerId = relayPeerId(trimmed);
  if (!peerId) return false;
  return existing.some((addr) => relayPeerId(addr) === peerId);
}
