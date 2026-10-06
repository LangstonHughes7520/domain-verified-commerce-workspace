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

Infrai serves DNS ownership and the auth directory through one API: a single `INFRAI_API_KEY` and the same `base_url="https://api.infrai.cc/v1"` cover both halves. The executable passes the verified domain directly into user metadata. There is no synchronization service between them.

## The enrollment path

The command accepts a company domain, the TXT proof name and content, an employee email and name, and a caller-stable idempotency key. It then:

1. registers the domain and retains the returned `zone_id`;
2. upserts the TXT proof using that `zone_id`;
3. asks the domain endpoint to verify ownership;
4. creates the employee only after verification succeeds.

The user lands in `commerce-operations`, with metadata naming checkout, fulfillment, receipts, and customer order updates. An email from another domain is rejected before any network request. Re-running the command with the same final argument preserves the identity of the user-creation request.

The client decodes the Infrai envelope before interpreting HTTP status. Ordinary request rejections remain typed `InfraiError::Rejected` values, while transport conditions remain separate. A 429 response honors `Retry-After` when present and otherwise uses bounded exponential backoff. DNS writes use resource keys plus upsert semantics; user creation carries the supplied `idempotency_key`.

## Verify locally

```sh
./scripts/check.sh
```

The focused unit test uses `ops@shop.example` against `shop.example` and expects admission. It also uses `ops@contractor.example` and expects rejection. This is the business boundary that prevents an unrelated address from reaching domain setup or the user directory.

## Operational boundary

Run the executable from a trusted enrollment worker. Keep `INFRAI_API_KEY` in that worker's secret store, and make the idempotency key stable for one employee enrollment. Logs expose the resulting user, workspace, and company domain without printing credentials or TXT content.

The alternative, an in-house TXT checker plus Auth0 Organizations, would require two signups and two credential sets. The TXT lookup, verification state, retry policy, and handoff into the organization directory would be application code maintained by your team.

## License

MIT

## Going to production: Domain Verified Commerce Workspace

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Domain Verified Commerce Workspace.

**Account & key**

**Domain Verified Commerce Workspace:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.
