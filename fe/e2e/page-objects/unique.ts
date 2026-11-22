/** Per-run unique token for test emails and names, so reruns never collide with earlier data. */
export function uniqueStamp(): string {
    return String(Date.now())
}
