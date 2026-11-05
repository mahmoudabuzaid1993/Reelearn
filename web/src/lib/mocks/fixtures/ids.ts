/** Deterministic UUIDs so fixtures, tests and the API seed can line up. */
export function uid(block: number, n: number): string {
  return `00000000-0000-4000-8000-${String(block).padStart(4, "0")}${String(n).padStart(8, "0")}`;
}

export const IDS = {
  tenant: uid(1, 1),
  users: {
    noura: uid(2, 1),
    khalid: uid(2, 2),
    hana: uid(2, 3),
    sara: uid(2, 4),
    rizwan: uid(2, 5),
    mohammad: uid(2, 6),
    fatimah: uid(2, 7),
  },
  groups: { store12: uid(3, 1), warehouse: uid(3, 2), allStaff: uid(3, 3) },
  topics: { customerService: uid(4, 1), storeSafety: uid(4, 2), aml: uid(4, 3) },
  reels: {
    customerName: uid(5, 1),
    spill: uid(5, 2),
    cashReports: uid(5, 3),
    greeting: uid(5, 4),
    ladder: uid(5, 5),
    kyc: uid(5, 6),
    podcastCut: uid(5, 7),
  },
  sources: { handbook: uid(6, 1), podcast: uid(6, 2), youtube: uid(6, 3), prompt: uid(6, 4) },
  jobs: { done: uid(7, 1), failed: uid(7, 2) },
  moments: { m1: uid(8, 1), m2: uid(8, 2), m3: uid(8, 3), m4: uid(8, 4) },
  exports: { q3: uid(9, 1) },
  questions: { customerName: uid(10, 1), spill: uid(10, 2), cash: uid(10, 3), greeting: uid(10, 4), ladder: uid(10, 5), kyc: uid(10, 6) },
} as const;
