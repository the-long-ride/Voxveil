## Summary

Describe the user-visible or architectural change and why it is needed.

## Verification

- [ ] Tests added or updated before implementation where behavior changed.
- [ ] Frontend and Rust coverage remain at or above 85%.
- [ ] LOC gate passes: `.ts <= 300`, `.tsx <= 400`, `.rs <= 300`, `.css <= 400`.
- [ ] No unnecessary dependency was added.
- [ ] Every new dependency has exact-version, commercial-license, and source review.
- [ ] No telemetry, analytics, remote font, or general-purpose network behavior was introduced.
- [ ] i18n resources were updated for all seven locales when UI copy changed.
- [ ] Light and dark themes were checked.
- [ ] Responsive layout was checked at desktop and 320px mobile width.
- [ ] Relevant `/docs/specs` documents were updated.
- [ ] Security/privilege implications were reviewed.
- [ ] Calls/VoIP remain bypassed by default unless this PR explicitly changes the approved spec.

## Dependency review

If dependencies changed, list each package/crate, exact version, license, source, and why local code is not preferable.
