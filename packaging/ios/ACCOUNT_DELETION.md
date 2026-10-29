# OpenPencil account deletion

This feature deletes the OpenPencil product account. It preserves the shared
ZSeven sign-in identity and other ZSeven products. Local source validation is
not evidence that the new service endpoints or the replacement app build are
deployed.

## User flow

1. In the iOS editor, open **More > Account > Delete OpenPencil Account**.
2. Read the cloud and local deletion scope. The app opens the dedicated SSO
   page in an in-app Safari sheet. Sign in again when recent authentication
   is required, verify the displayed account, and confirm deletion.
3. The service records a durable deletion job and blocks new OpenPencil
   access. Both regional services must acknowledge cleanup. Previously
   issued collaboration credentials require a 16-minute drain interval.
   Outages can extend processing; an accepted request is not completion.
4. The app verifies completion over HTTPS with its private receipt before
   removing app-owned local data. Closing Safari, signing out, a callback,
   an expired receipt, or an HTTP error cannot authorize local erasure.
5. After cleanup, the app shows a completion/restart screen. Relaunch it
   normally to start with clean local storage. The app never calls `exit(0)`.

The pending receipt survives app restarts. An expired receipt can be renewed
through identity verification without canceling or recreating the server
job. A partial local cleanup retains its recovery marker and blocks editor
startup until cleanup succeeds.

## Data boundary

| Removed or disabled | Preserved |
| --- | --- |
| Regional Hub session/profile copies, capture inbox, access-statistics profile, MCP credentials, explicitly OpenPencil-scoped Hosted credentials | Shared ZSeven user, email, password, linked identity providers, and other product sessions |
| New OpenPencil collaboration and product-scoped Hosted access | Shared wallet, financial records, generic shared-service authorization and usage records |
| This installation's OpenPencil-managed documents, autosaves, settings, bookmarks, local credentials and caches | Files exported outside this app's container, collaborators' copies, and files on other devices |

A minimal product tombstone remains to reject old OpenPencil access; normal
sign-in must not silently recreate the deleted product account. Shared
identity and provider bindings are deliberately not deleted or globally
revoked. Historical LLM tokens lack product ownership, so they cannot be
bulk-erased without affecting other products. Older released clients can
still use that generic shared-service path; do not describe it as revoked.

## Completion contract

- Native creation sends the current device ID and SHA-256 of a random
  receipt secret to `/api/v1/openpencil/deletion-requests`. Only the hash
  is stored server-side. The secret never appears in a URL or log.
- The web review/confirmation routes check device ownership against the
  authenticated SSO user and require authentication within 15 minutes for
  confirmation. The confirmation literal is `DELETE_OPENPENCIL`.
- Native status requests carry the receipt secret in `Authorization`.
  Response request ID, device ID, scope and pinned origin must all match.
  Only `state: deleted` authorizes local cleanup.
- Unconfirmed requests last 30 minutes; confirmation extends the receipt
  to seven days. New verified receipts can bind the same existing job.
- Regional cleanup acknowledgements must match the exact subject, job,
  scope and region. A redirect, partial response or one successful region
  is insufficient.
- Hub write permits stop new writes and drain admitted writes before
  cleanup. Permits have no timeout that could let a slow writer resurrect
  deleted data. Crash recovery requires the documented reconciliation tool
  and proof that the exact recorded process has stopped.
- Local cleanup stops all editor scenes and pending save work before
  removing files. It must not follow symlinks into external storage or
  delete shared browser cookies.

## Release dependencies

Complete these steps in the service repositories before submitting a build:

1. Apply SSO migration `000010_openpencil_accounts` before deploying the
   product guards. Existing shared-identity tables remain intact.
2. Deploy the SSO status reader and the CN/Global Hub cleanup handlers with
   dedicated mounted status/cleanup credentials. Verify exact region
   acknowledgements and partial-failure recovery. Do not reuse billing,
   canvas, LLM issuer, or user credentials.
3. Deploy the Gateway change that bounds inference by credential expiry,
   and the product-scoped Hosted endpoint/token implementation.
4. Enable new deletion intake only after both cleanup services are ready.
   Disabling intake must preserve existing tombstones, status checks and
   job retries. Follow the service-specific configuration/runbooks.
5. Rebuild, audit, sign and adopt the private auth matrix. OpenPencil now
   requires the explicit Hosted product claim; the old signed archive
   fails closed for Hosted access. Do not relax signature verification.
6. Publish the reviewed bilingual privacy-policy update and ship the new
   iOS build. Source edits do not update the deployed page or old binaries.
7. Use disposable accounts on physical devices to verify registration,
   login, both-region cleanup, completion recovery and local erasure.
   Confirm the same ZSeven identity still works in another product and
   that its shared wallet is unchanged.

Use [AppReviewRecordingChecklist.md](AppReviewRecordingChecklist.md) for the
recording and [AppReviewNotes.txt](AppReviewNotes.txt) for the six requested
review items. Never use the reviewer login account for the deletion demo.

## Local verification, 2026-10-29

- iOS: full source/ABI validation, Swift warnings-as-errors, receipt binding,
  recovery, symlink safety and asynchronous save-drain regression tests.
- SSO: affected Go packages with the race detector; full migration
  up/down/restore checks and concurrent lifecycle tests against disposable
  MySQL 8.4. The disposable database/container was removed afterward.
- Hub: full backend test suite and affected-package race tests, including
  both-region acknowledgement binding, concurrent writes, shared-token
  preservation, Redis failover scanning and stopped-process recovery.
- Web confirmation: 518 frontend tests, typecheck, lint, build and a mocked
  390-pixel browser layout check. This is not physical-device QA.
- Hosted: product wire/runtime tests and Clippy. Existing signed provenance
  tests passed, but they do not sign or adopt the newly changed runtime.
- Gateway: HTTP tests, race tests and vet; relay tests prove credential
  expiry closes both waiting and paired connections.

These checks do not prove live cross-service deletion, a production rollout,
App Store approval or physical-device behavior. No account or production
data was deleted during this work.
