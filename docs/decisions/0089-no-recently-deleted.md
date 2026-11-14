# 0089 — No Recently deleted: a deletion is for good

**Status:** Accepted and built, 2026-11-14. Supersedes
[0042](0042-recently-deleted.md). Amends
[0043](0043-settling-a-conflict.md) (the version not kept was put in
Recently deleted) and [0073](0073-the-phone-works-with-its-computer.md)
(a phone's deletions in its folder on a computer, put back from either).
Built and tested by the suites; **not yet tried on the phones**.

## What was asked

The owner, 2026-11-14: "also remove the recently deleted from both phone and
laptop", and, asked whether that meant emptying it, hiding it, or removing the
feature: remove the feature.

## Decided

- **Nothing shows deleted files or puts them back.** Gone: the phone's
  *Recently deleted* page (`DeletedScreen.kt`) and its Settings row, the
  window's page, its Settings row, its storage row and Files' *Deleted*
  button, the computer's row on the phone's storage page and its *Empty*,
  and `qurb deleted`, `qurb restore`, `qurb forget`. The engine functions
  that served them went with them.
- **A deletion still passes through the store's `trash/`**, as before, and
  housekeeping empties it whatever its age: every five minutes in the
  daemon, after each sync on a phone (`Engine::housekeep`, which empties it
  before collecting). A delete made in either app empties its own entry at
  once. A deletion that arrives from another device, or one a phone makes on
  its computer's shared folder, goes at that device's next housekeeping.
- **A deleted Private Vault file's chunks** are no longer held thirty days on
  the computer (`gc::collect`); they go with the ordinary retention window,
  seven days in the daemon.
- **The protocol keeps** `Request::EmptyDeleted` and `Part::Deleted`, answered
  and read for builds before this; a computer no longer reports the part.

Why through `trash/` rather than unlinking at once: the store's bookkeeping
for a deleted file -- its counted size, a held file's chunks, a vault file
freed to a computer -- is all on that path and tested there. Taking the path
away would rewrite deletion across the store for no difference a person sees.

## Consequences

- A file deleted by mistake, on any device, cannot be got back from QurbSpace
  once housekeeping has run. Only a computer's own file manager's trash, for
  a file deleted there, still has it.
- Settling a conflict deletes the version not kept: *Keep both* is the only
  choice that loses nothing.
- Deleted files stop taking space within minutes.

## Left undone

- The store's restore paths (`restore_from_trash`, `restore_freed`,
  `restore_held`, `take_back_restored`) and their tests remain, reached from
  no window. Removing them is a rewrite of the store's deletion, left unless
  wanted.
- Not tried on the S23 or the OPPO.
