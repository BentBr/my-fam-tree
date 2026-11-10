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
