export const PROJECT_NAME = "TokenWatch";

// ── Domain / DNS ─────────────────────────────────────────────────────────────
// DnsStack imports an existing Route53 hosted zone (it does NOT create one) and
// owns the wildcard ACM certificate. Before deploying:
//   1. Register the domain and create its hosted zone in Route53.
//   2. Set HOSTED_ZONE_ID below to the real zone id (currently a placeholder).
export const DOMAIN_NAME = "tokenwatch.gulloa.click";
export const HOSTED_ZONE_ID = "Z02980033T4KKO41XQM55";
export const RELEASES_SUBDOMAIN = `releases.${DOMAIN_NAME}`;
export const RELEASES_PUBLIC_URL = `https://${RELEASES_SUBDOMAIN}`;

export const LANDING_DOMAIN = DOMAIN_NAME;
export const LANDING_WWW_SUBDOMAIN = `www.${DOMAIN_NAME}`;
export const LANDING_PUBLIC_URL = `https://${DOMAIN_NAME}`;

export const FEEDBACK_SUBDOMAIN = `feedback.${DOMAIN_NAME}`;
export const FEEDBACK_PUBLIC_URL = `https://${FEEDBACK_SUBDOMAIN}`;

// ── Analytics ────────────────────────────────────────────────────────────────
export const ANALYTICS_LOG_BUCKET_SSM = "/TokenWatch/analytics/log-bucket-name";
export const LANDING_LOG_PREFIX = "landing/";
export const RELEASES_LOG_PREFIX = "releases/";
export const ANALYTICS_GLUE_DATABASE = "tokenwatch_analytics";
export const ANALYTICS_WORKGROUP = "tokenwatch-analytics";
export const ANALYTICS_LOG_RETENTION_DAYS = 90;

// ── Feedback ─────────────────────────────────────────────────────────────────
/** SSM path for the rotatable app-key (stored as a SecureString by the operator). */
export const FEEDBACK_APP_KEY_SSM = "/TokenWatch/feedback/app-key";

/** Maximum bytes per attachment file (5 MB). */
export const FEEDBACK_MAX_ATTACHMENT_BYTES = 5 * 1024 * 1024; // 5 MB
/** Maximum number of attachment files per submission. */
export const FEEDBACK_MAX_ATTACHMENTS = 3;
/** Maximum characters in the feedback message body. */
export const FEEDBACK_MAX_MESSAGE_CHARS = 5000;
