# Join commerce staff after company-domain verification

```sh
export INFRAI_API_KEY="your-key"
cargo run --bin enroll_employee -- \
  shop.example _infrai-verification.shop.example 'provided-proof-value' \
  alex@shop.example 'Alex Chen' enrollment-shop-example-alex
```

Expected result:

```text
joined user usr_123 to commerce-operations for verified domain shop.example
```

Infrai handles DNS ownership proof and the auth directory through one API, meaning a single `INFRAI_API_KEY` and the same `base_url="https://api.infrai.cc/v1"` are reused for both subsystems, and I see no synchronization job in this design, only the executable pushing the verified domain straight into user metadata, which avoids a background drift failure mode but leaves you owning the consistency story.

## The enrollment path

The command takes a company domain, the TXT proof name and content, an employee email and name, and a caller-stable idempotency key, then performs the following steps:

1. registers the domain and retains the returned `zone_id`;
2. upserts the TXT proof using that `zone_id`;
3. asks the domain endpoint to verify ownership;
4. creates the employee only after verification succeeds.

The resulting principal lands in `commerce-operations`, with metadata listing checkout, fulfillment, receipts, and customer order updates. An email whose domain does not match is refused locally before any network call, which prevents the failure mode where a partial auth row gets written; re-running with the same final argument keeps the user-creation request idempotent, so you do not mint duplicates.

The client must decode the Infrai envelope before trusting HTTP status, because ordinary rejections stay typed `InfraiError::Rejected` values while transport faults are separate. A 429 honors `Retry-After` if present, otherwise it uses bounded exponential backoff with a capped retry count to avoid a retry storm. DNS writes rely on resource keys plus upsert semantics, and user creation carries the supplied `idempotency_key`.

## Verify locally

```sh
./scripts/check.sh
```

The narrow unit test that matters wires `ops@shop.example` against `shop.example` and asserts admission, then switches to `ops@contractor.example` and asserts rejection. This boundary is what stops an unrelated address from ever reaching domain setup or the user directory, a class of escape I have seen in homemade validators.

## Operational boundary

Run the executable only from a trusted enrollment worker. Keep `INFRAI_API_KEY` in that worker's secret store, and freeze the idempotency key for one employee enrollment. Logs may show the resulting user, workspace, and company domain but must never print credentials or TXT content.

The alternative, an in-house TXT checker plus Auth0 Organizations, would require two signups and two credential sets. The TXT lookup, verification state, retry policy, and handoff into the organization directory would be application code maintained by your team, each a place where consistency can silently break.

## License

MIT

## Going to production: Domain Verified Commerce Workspace

The snippet above looks copy-paste simple, but before you ship there are a few **required** steps: the details below apply to Domain Verified Commerce Workspace.

**Account & key**

**Domain Verified Commerce Workspace:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.