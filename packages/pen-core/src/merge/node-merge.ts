// packages/pen-core/src/merge/node-merge.ts
//
// Pure 3-way merge of PenDocument trees. Returns a best-effort merged document
// plus arrays of node-level and document-field-level conflicts that callers
// (the git engine and panel UI) resolve interactively.
//
// The merge module never assigns conflict ids — that's the git engine's job
// when wrapping the result into an IPC ConflictBag.
//
// Filled in by Task 7.

export {};
