# TokenWatch Infrastructure

AWS CDK infrastructure for the TokenWatch project.

## Stacks

| Stack | Description |
|---|---|
| `TokenWatchDnsStack` | Imports the existing Route53 hosted zone (does not create one) and owns the wildcard ACM certificate (`tokenwatch.app` + `*.tokenwatch.app`). Exports zone/cert/releases-url via SSM. |
| `TokenWatchAnalyticsStack` | Shared CloudFront access-log S3 bucket, Glue database + tables, Athena workgroup and named queries. |
| `TokenWatchReleasesStack` | S3 artifact bucket + CloudFront distribution (`releases.tokenwatch.app`) for binary releases. GitHub OIDC publish role. |
| `TokenWatchLandingStack` | Vite SPA (landing page) served via CloudFront (`tokenwatch.app` + `www`) from a private S3 bucket. |
| `TokenWatchFeedbackStack` | HTTP API (API Gateway v2) on `feedback.tokenwatch.app` + Lambda intake + DynamoDB + S3 attachments bucket for in-app feedback. |

## DnsStack / custom domain

`DnsStack` imports an existing hosted zone — it does **not** create one. Before
deploying:

1. Register the domain and create its Route53 hosted zone.
2. Set `HOSTED_ZONE_ID` in `constants.ts` to the real zone id (it currently ships
   with a `ZXXXXXXXXXXXXX` placeholder). `DOMAIN_NAME` is `tokenwatch.app`.

`ReleasesStack`, `LandingStack` and `FeedbackStack` read the zone + certificate
from `DnsStack` via SSM and add their `domainNames`/`certificate` and Route53
alias records.

## Development

```bash
pnpm install
pnpm run build       # type-check
pnpm run test        # jest
pnpm run synth       # cdk synth
```

## Scripts

- `scripts/set-feedback-app-key.sh` — provision or rotate the feedback app-key in SSM and GitHub Actions.
