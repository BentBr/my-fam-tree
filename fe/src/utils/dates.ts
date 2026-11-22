// Bounds for genealogical `<input type="date">` fields (birth, death,
// partnership start/end).
//
// Always set `min` explicitly: without it, some native pickers fall back
// to their own default range. Android's DatePicker (used by Firefox for
// Android) starts at 1900-01-01, so an 1870 ancestor can't be picked and
// the browser shows its own "invalid value" message.
//
// Mirrors `MIN_DATE_YEAR` in `crates/api/src/validation/mod.rs`; the API
// rejects anything earlier with `validation.date_too_early`.
export const MIN_DATE = '1000-01-01'

/** `YYYY-MM-DD` in local time — the `<input type="date">` format, without `toISOString()`'s UTC day shift. */
export function toLocalIsoDate(d: Date): string {
    const year = String(d.getFullYear()).padStart(4, '0')
    const month = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    return `${year}-${month}-${day}`
}
