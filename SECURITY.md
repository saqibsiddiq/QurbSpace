# Security

QurbSpace handles people's private files, so a security problem matters more
than any other kind.

## Reporting

**Please do not open a public issue.** Use GitHub's
[private vulnerability reporting](https://github.com/saqibsiddiq/QurbSpace/security/advisories/new),
or email **sddqsaqib@gmail.com** with "QurbSpace security" in the subject.

Say what you found, how to reproduce it, and what it lets someone do. You
will hear back within a few days.

## Scope

The apps (Android, Linux, Windows), the engine in `crates/`, and the release
and update path. The public Nostr relays, STUN servers, Tor and Google Drive
are other people's services; a problem in how QurbSpace *uses* them is in
scope.

## Status

This is an open beta. The design is described in
[docs/CODEBASE.md](docs/CODEBASE.md) and the decision records in
[docs/decisions/](docs/decisions/). **No independent security review has
been done yet.**
