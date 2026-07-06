import { type ReactNode, type ComponentType, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { PrivacyPolicy, TermsOfService } from "./legal";

/* ── Release manifest ──────────────────────────────────────────────────── */

const MANIFEST_URL = "https://releases.tokenwatch.gulloa.click/download.json";

type Installer = { url: string; filename: string; size: number };
type Manifest = {
  version: string;
  pub_date: string;
  installers: Record<string, Installer>;
};

type ManifestState =
  | { status: "loading" }
  | { status: "ready"; manifest: Manifest }
  | { status: "error" };

function fmtSize(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function fmtDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return iso.slice(0, 10);
  }
}

/** Best-effort OS-family detection: 'mac' | 'windows' | 'linux'. Defaults to 'mac'. */
function detectOS(): "mac" | "windows" | "linux" {
  try {
    // Modern hint API (Chrome 90+, Edge 90+)
    const platform = (navigator as Navigator & { userAgentData?: { platform?: string } })
      .userAgentData?.platform?.toLowerCase() ?? "";
    if (platform.includes("win")) return "windows";
    if (platform.includes("linux")) return "linux";
    if (platform.includes("mac")) return "mac";

    // Legacy UA string
    const ua = navigator.userAgent.toLowerCase();
    if (/windows/.test(ua)) return "windows";
    if (/linux/.test(ua) && !/android/.test(ua)) return "linux";
    if (/mac/.test(ua)) return "mac";

    // Legacy platform string
    const leg = (navigator.platform ?? "").toLowerCase();
    if (/win/.test(leg)) return "windows";
    if (/linux/.test(leg)) return "linux";
    if (/mac/.test(leg)) return "mac";
  } catch {
    /* ignore */
  }
  return "mac";
}

/** Best-effort: is this an Apple-Silicon Mac? Falls back to true (the common case). */
function prefersAppleSilicon(): boolean {
  try {
    const canvas = document.createElement("canvas");
    const gl =
      (canvas.getContext("webgl") as WebGLRenderingContext | null) ||
      (canvas.getContext("experimental-webgl") as WebGLRenderingContext | null);
    if (gl) {
      const dbg = gl.getExtension("WEBGL_debug_renderer_info");
      if (dbg) {
        const r = String(
          gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) || ""
        ).toLowerCase();
        if (r.includes("intel")) return false;
        if (r.includes("apple")) return true;
      }
    }
  } catch {
    /* ignore */
  }
  return true; // most new Macs are Apple Silicon
}

/* ── Icons (hairline, currentColor — per DESIGN.md) ────────────────────── */

const Eye = ({ size = 26 }: { size?: number }) => (
  <img className="mark" src="/logo.svg" width={size} height={size} alt="TokenWatch" />
);

const AppleLogo = ({ size = 18 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor">
    <path d="M17.05 12.04c-.03-2.8 2.29-4.15 2.39-4.21-1.3-1.9-3.33-2.16-4.05-2.19-1.72-.17-3.36 1.01-4.23 1.01-.87 0-2.22-.99-3.65-.96-1.88.03-3.61 1.09-4.58 2.77-1.95 3.39-.5 8.41 1.4 11.16.93 1.35 2.04 2.86 3.49 2.81 1.4-.06 1.93-.9 3.62-.9 1.69 0 2.17.9 3.65.87 1.51-.03 2.46-1.37 3.38-2.73 1.07-1.57 1.51-3.09 1.53-3.17-.03-.01-2.94-1.13-2.97-4.47zM14.6 4.04c.77-.93 1.29-2.23 1.15-3.52-1.11.05-2.46.74-3.25 1.67-.71.83-1.33 2.15-1.16 3.42 1.24.1 2.5-.63 3.26-1.57z" />
  </svg>
);

const WindowsLogo = ({ size = 18 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 5.5 11.5 4v7.5H3z" />
    <path d="M13 3.7 21 2.5v9H13z" />
    <path d="M3 12.5h8.5V20L3 18.5z" />
    <path d="M13 12.5H21V21l-8-1.2z" />
  </svg>
);

const LinuxLogo = ({ size = 18 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2C9 2 7 4.5 7 8c0 2 .5 3.5 1 5l-2 3c-.5 1 0 2 1 2h10c1 0 1.5-1 1-2l-2-3c.5-1.5 1-3 1-5 0-3.5-2-6-5-6z" />
    <path d="M9.5 14.5c-.5.5-1 1-1 1.5M14.5 14.5c.5.5 1 1 1 1.5" />
    <circle cx="10" cy="9" r="1" fill="currentColor" stroke="none" />
    <circle cx="14" cy="9" r="1" fill="currentColor" stroke="none" />
  </svg>
);

const Download = ({ size = 18 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 3v12m0 0 4-4m-4 4-4-4" />
    <path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
  </svg>
);

const Arrow = ({ size = 15 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
    <path d="M5 12h14m-6-6 6 6-6 6" />
  </svg>
);

const Chip = ({ size = 12 }: { size?: number }) => (
  <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <rect x="6" y="6" width="12" height="12" rx="1.5" />
    <path d="M9 1v3M15 1v3M9 20v3M15 20v3M1 9h3M1 15h3M20 9h3M20 15h3" />
  </svg>
);

/* ── OS-aware installer selection ──────────────────────────────────────── */

type OsFamily = "mac" | "windows" | "linux";

/** Ordered platform keys to try for a given OS, most-preferred first. */
function platformKeys(os: OsFamily, arm: boolean): string[] {
  if (os === "mac") return arm ? ["darwin-aarch64", "darwin-x86_64"] : ["darwin-x86_64", "darwin-aarch64"];
  if (os === "windows") return ["windows-x86_64", "darwin-aarch64", "darwin-x86_64", "linux-x86_64"];
  return ["linux-x86_64", "darwin-aarch64", "darwin-x86_64", "windows-x86_64"];
}

/** Returns the best available installer for the detected OS, never undefined if any exist. */
function pickPrimary(
  installers: Record<string, Installer>,
  os: OsFamily,
  arm: boolean,
): { key: string; installer: Installer } | undefined {
  const keys = platformKeys(os, arm);
  for (const k of keys) {
    if (installers[k]) return { key: k, installer: installers[k] };
  }
  // Final fallback: any available installer
  const first = Object.entries(installers)[0];
  return first ? { key: first[0], installer: first[1] } : undefined;
}

/* ── Skeleton helpers ──────────────────────────────────────────────────── */

function Skel({ w, h, className }: { w?: string; h?: string; className?: string }) {
  return (
    <span
      className={`skeleton${className ? ` ${className}` : ""}`}
      style={{ width: w, height: h }}
    />
  );
}

function DownloadCardSkeleton() {
  return (
    <div className="dl-card">
      {/* arch line */}
      <div className="arch">
        <Skel w="18px" h="18px" />
        <Skel w="120px" h="16px" />
      </div>
      {/* sub */}
      <Skel w="90px" h="13px" className="dl-card-skel-sub" />
      {/* meta */}
      <div className="meta" style={{ gap: 6 }}>
        <Skel w="160px" h="11px" />
        <Skel w="100px" h="11px" />
      </div>
      {/* button footprint */}
      <span className="dl-btn dl-btn-skeleton" aria-hidden="true">
        <Skel w="16px" h="16px" />
        <Skel w="64px" h="14px" />
      </span>
    </div>
  );
}

/* ── Reveal-on-scroll hook ─────────────────────────────────────────────── */

function useReveal() {
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      document.querySelectorAll(".reveal").forEach((el) => el.classList.add("in"));
      return;
    }
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("in");
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.12 }
    );
    document.querySelectorAll(".reveal").forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, []);
}

/* ── App ───────────────────────────────────────────────────────────────── */

export default function App() {
  const path = typeof window !== "undefined" ? window.location.pathname.replace(/\/+$/, "") : "";

  const [state, setState] = useState<ManifestState>({ status: "loading" });
  const isArm = useRef<boolean>(true);
  const os = useRef<OsFamily>("mac");
  useReveal();

  const loadManifest = useCallback(() => {
    setState({ status: "loading" });
    let cancelled = false;
    fetch(MANIFEST_URL, { cache: "no-cache" })
      .then((r) => (r.ok ? r.json() : Promise.reject()))
      .then((m: Manifest) => {
        if (!cancelled && m?.installers) setState({ status: "ready", manifest: m });
        else if (!cancelled) setState({ status: "error" });
      })
      .catch(() => {
        if (!cancelled) setState({ status: "error" });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    os.current = detectOS();
    isArm.current = prefersAppleSilicon();
    return loadManifest();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Derive manifest only when ready
  const manifest = state.status === "ready" ? state.manifest : null;

  // Derive the list of present installers in display order (only those in manifest)
  const PLATFORM_ORDER = ["darwin-aarch64", "darwin-x86_64", "windows-x86_64", "linux-x86_64"];
  const presentInstallers = useMemo(
    () => PLATFORM_ORDER.filter((k) => !!(manifest?.installers ?? {})[k]),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [manifest?.installers],
  );

  // Resolve the primary (recommended) installer for hero CTA
  const primaryResult = useMemo(
    () => pickPrimary(manifest?.installers ?? {}, os.current, isArm.current),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [manifest?.installers],
  );
  const primary = primaryResult?.installer;
  const primaryKey = primaryResult?.key ?? "darwin-aarch64";

  // Legal page routing — after all hooks to satisfy Rules of Hooks
  if (path === "/privacy") return (
    <>
      <div className="atmosphere" />
      <div className="grain" />
      <div className="shell"><PrivacyPolicy /></div>
    </>
  );
  if (path === "/terms") return (
    <>
      <div className="atmosphere" />
      <div className="grain" />
      <div className="shell"><TermsOfService /></div>
    </>
  );

  // Per-card meta: title, sub, glyph, ext
  function cardMeta(key: string): { title: string; sub: string; Glyph: ComponentType<{ size?: number }>; ext: string } {
    if (key === "darwin-aarch64") return { title: "Apple Silicon", sub: "M1, M2, M3 and later",     Glyph: AppleLogo, ext: ".dmg" };
    if (key === "darwin-x86_64")  return { title: "Intel",         sub: "x86-64 Macs",              Glyph: AppleLogo, ext: ".dmg" };
    if (key === "windows-x86_64") return { title: "Windows",       sub: "x86-64 · Windows 10+",     Glyph: WindowsLogo, ext: ".msi" };
    return                               { title: "Linux",         sub: "x86-64 AppImage",           Glyph: LinuxLogo,   ext: ".AppImage" };
  }

  return (
    <>
      <div className="atmosphere" />
      <div className="grain" />

      <div className="shell">
        {/* ── Nav ── */}
        <nav className="nav">
          <div className="container nav-inner">
            <a className="brand" href="#top">
              <Eye />
              TokenWatch
            </a>
            <div className="nav-links">
              <a href="#how">How it works</a>
              <a href="#dashboard">The dashboard</a>
              <a href="#features">Features</a>
              <span className="version-pill">
                <span className="dot" />
                {state.status === "ready"
                  ? `v${manifest!.version}`
                  : <Skel w="48px" h="11px" />
                }
              </span>
              <a className="nav-cta" href="#download">
                Download
              </a>
            </div>
          </div>
        </nav>

        {/* ── Hero ── */}
        <header className="hero" id="top">
          <div className="scan" />
          <div className="container hero-grid">
            <div className="hero-copy">
              <span className="hero-tag">
                <span className="chip">TokenWatch</span>
                Your fuel gauge for Claude
              </span>
              <h1>
                Never get{" "}
                <span className="accent-word">rate-limited</span>{" "}
                mid-task again.
              </h1>
              <p className="lede">
                TokenWatch lives in your macOS menu bar and monitors Claude token usage per project and workspace. Live session (5h) and weekly limit gauges, per-group budgets, and alerts before you hit the cap — no cloud, no telemetry.
              </p>

              <div className="cta-row">
                {state.status === "ready" ? (
                  <a className="btn-download" href={primary?.url}>
                    <AppleLogo />
                    <span className="bd-text">
                      Download for macOS
                      <span className="bd-meta">
                        .dmg · {primary ? fmtSize(primary.size) : "—"}
                      </span>
                    </span>
                  </a>
                ) : (
                  <span className="btn-download btn-download-skeleton" aria-hidden="true">
                    <Skel w="18px" h="18px" />
                    <span className="bd-text">
                      <Skel w="160px" h="15px" />
                      <Skel w="100px" h="11px" className="bd-meta-skel" />
                    </span>
                  </span>
                )}
                <a className="btn-ghost" href="#how">
                  How it works <Arrow />
                </a>
              </div>

              <div className="cta-note">
                <span>Free</span>
                <span className="sep" />
                {state.status === "ready" ? (
                  <>
                    <span>v{manifest!.version}</span>
                    <span className="sep" />
                    <span>Built {fmtDate(manifest!.pub_date)}</span>
                  </>
                ) : (
                  <>
                    <Skel w="48px" h="11px" />
                    <span className="sep" />
                    <Skel w="96px" h="11px" />
                  </>
                )}
              </div>
            </div>

            {/* Popover Mockup */}
            <div className="mockup-wrap">
              <PopoverMockup />
            </div>
          </div>
        </header>

        {/* ── How it works ── */}
        <section className="section" id="how">
          <div className="container">
            <div className="section-head reveal">
              <span className="eyebrow">
                <span className="accent">01</span> &nbsp;How it works
              </span>
              <h2>Reads your logs. Never touches the network.</h2>
              <p>
                TokenWatch parses Claude Code's local logs and computes your usage. No keys to configure, no telemetry, no cloud — your data never leaves your Mac.
              </p>
            </div>
            <div className="sources reveal">
              <div className="source">
                <span className="source-idx">01</span>
                <div className="source-name">Reads <code>~/.claude/projects</code></div>
                <div className="source-desc">Parses the <code>.jsonl</code> from each Claude Code session and dedupes by message. All processing happens on your machine.</div>
              </div>
              <div className="source">
                <span className="source-idx">02</span>
                <div className="source-name">Refreshes every <code>30 s</code></div>
                <div className="source-desc">Incremental ingest: only re-reads files that changed. Near-zero overhead in the background.</div>
              </div>
              <div className="source">
                <span className="source-idx">03</span>
                <div className="source-name">Groups by project and model</div>
                <div className="source-desc">Attributes tokens and cost to each workspace, with normalized Conductor names.</div>
              </div>
              <div className="source">
                <span className="source-idx">04</span>
                <div className="source-name">Prices Opus · Sonnet · Haiku</div>
                <div className="source-desc">Converts tokens —including cache— to USD using each model's current rate table.</div>
              </div>
            </div>
          </div>
        </section>

        {/* ── The dashboard ── */}
        <section className="section" id="dashboard">
          <div className="container">
            <div className="section-head reveal">
              <span className="eyebrow">
                <span className="accent">02</span> &nbsp;The dashboard
              </span>
              <h2>Every token, on its chart.</h2>
              <p>
                Open the cost dashboard for historical analysis: stacked time series by model or by project, range filters (24 h, 7 d, 30 d, month, or custom), and a table that balances like a ledger.
              </p>
            </div>
            <div className="mockup-wrap reveal dashboard-wrap">
              <DashboardMockup />
            </div>
          </div>
        </section>

        {/* ── Features ── */}
        <section className="section" id="features">
          <div className="container">
            <div className="section-head reveal">
              <span className="eyebrow">
                <span className="accent">03</span> &nbsp;What's inside
              </span>
              <h2>A cockpit instrument for your quota.</h2>
            </div>

            <div className="bento reveal">
              {/* Live limits — span-3 */}
              <div className="feat span-3">
                <span className="ftag">Live limits</span>
                <h3>Session and week gauges</h3>
                <p>
                  Reads your 5-hour and weekly limits from Anthropic's API. Rails with thresholds at 70/85/100 and a pace marker: you know at a glance whether you're cruising or at the edge.
                </p>
                <div className="feat-gauge-ornament">
                  <div className="fgo-label">SESSION 5H</div>
                  <div className="fgo-track">
                    <div className="fgo-fill" style={{ width: "72%", background: "var(--watch)" }} />
                    <div className="fgo-tick" style={{ left: "70%" }} />
                    <div className="fgo-tick" style={{ left: "85%" }} />
                    <div className="fgo-tick" style={{ left: "100%" }} />
                    <div className="fgo-pace" style={{ left: "68%" }} />
                  </div>
                  <div className="fgo-meta">
                    <span className="fgo-pct" style={{ color: "var(--watch)" }}>72%</span>
                    <span className="fgo-reset">Resets in 1h 24m</span>
                  </div>
                </div>
              </div>

              {/* By model — span-3 */}
              <div className="feat span-3">
                <span className="ftag">By model</span>
                <h3>Weekly breakdown Opus · Sonnet · Haiku</h3>
                <p>
                  Each model eats your weekly quota separately. See which one is burning through it before it stops you.
                </p>
                <div className="feat-model-gauges">
                  <div className="fmg-row">
                    <span className="fmg-name">Opus</span>
                    <div className="fmg-track">
                      <div className="fmg-fill" style={{ width: "84%", background: "var(--danger)" }} />
                      <div className="fmg-tick" style={{ left: "70%" }} />
                      <div className="fmg-tick" style={{ left: "85%" }} />
                    </div>
                    <span className="fmg-pct" style={{ color: "var(--danger)" }}>84%</span>
                  </div>
                  <div className="fmg-row">
                    <span className="fmg-name">Sonnet</span>
                    <div className="fmg-track">
                      <div className="fmg-fill" style={{ width: "61%", background: "var(--watch)" }} />
                      <div className="fmg-tick" style={{ left: "70%" }} />
                    </div>
                    <span className="fmg-pct" style={{ color: "var(--watch)" }}>61%</span>
                  </div>
                  <div className="fmg-row">
                    <span className="fmg-name">Haiku</span>
                    <div className="fmg-track">
                      <div className="fmg-fill" style={{ width: "22%", background: "var(--safe)" }} />
                      <div className="fmg-tick" style={{ left: "70%" }} />
                    </div>
                    <span className="fmg-pct" style={{ color: "var(--safe)" }}>22%</span>
                  </div>
                </div>
              </div>

              {/* Budgets — span-2 */}
              <div className="feat span-2">
                <span className="ftag">Budgets</span>
                <h3>Caps per project group</h3>
                <p>
                  Group projects and give them a cap: a percentage of the session or an absolute USD amount. Automatic alerts at 50, 70, and 80%.
                </p>
              </div>

              {/* Alerts — span-2 */}
              <div className="feat span-2">
                <span className="ftag">Alerts</span>
                <h3>A heads-up before the cap</h3>
                <p>
                  Native macOS notification as you approach the limit. Mute it with one tap when you're in focus mode.
                </p>
              </div>

              {/* Auto-updates — span-2 */}
              <div className="feat span-2">
                <span className="ftag">Always current</span>
                <h3>Automatic updates</h3>
                <p>
                  Updates itself from signed releases. Always the latest version, no friction.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* ── Download ── */}
        <section className="section dl-section" id="download">
          <div className="container">
            <div className="section-head reveal" style={{ marginInline: "auto", textAlign: "center", maxWidth: 560 }}>
              <span className="eyebrow">
                <span className="accent">04</span> &nbsp;Download
              </span>
              <h2>Download TokenWatch.</h2>
              <p>
                {state.status === "ready"
                  ? <>For macOS (Apple Silicon or Intel). Free, version {manifest!.version}.</>
                  : <>For macOS (Apple Silicon or Intel). Free.</>
                }
              </p>
            </div>

            <div className="dl-cards reveal">
              {state.status === "loading" && (
                <>
                  <DownloadCardSkeleton />
                  <DownloadCardSkeleton />
                </>
              )}
              {state.status === "ready" && presentInstallers.map((key) => {
                const { title, sub, Glyph, ext } = cardMeta(key);
                return (
                  <DownloadCard
                    key={key}
                    title={title}
                    sub={sub}
                    glyph={<Glyph size={18} />}
                    ext={ext}
                    installer={manifest!.installers[key]}
                    recommended={key === primaryKey}
                  />
                );
              })}
              {state.status === "error" && (
                <div className="dl-error">
                  <p>Downloads aren't available right now — reload or try again.</p>
                  <button
                    className="dl-retry-btn"
                    onClick={() => loadManifest()}
                    type="button"
                  >
                    Retry
                  </button>
                </div>
              )}
            </div>

            <div className="dl-foot">
              {state.status === "ready" ? (
                <>
                  <span>v{manifest!.version}</span>
                  <span>·</span>
                  <span>Released {fmtDate(manifest!.pub_date)}</span>
                  <span>·</span>
                </>
              ) : (
                <>
                  <Skel w="48px" h="11px" className="dl-foot-skel" />
                  <span>·</span>
                  <Skel w="100px" h="11px" className="dl-foot-skel" />
                  <span>·</span>
                </>
              )}
              <span>macOS 11+ · Apple Silicon or Intel</span>
            </div>
          </div>
        </section>

        {/* ── Footer ── */}
        <footer className="footer">
          <div className="container footer-inner">
            <a className="brand" href="#top">
              <Eye size={22} />
              TokenWatch
            </a>
            <div className="footer-meta">
              <a href="#how">How it works</a>
              <a href="#dashboard">The dashboard</a>
              <a href="#features">Features</a>
              <a href="#download">Download</a>
              <a href="/privacy">Privacy</a>
              <a href="/terms">Terms</a>
              {state.status === "ready"
                ? <span>v{manifest!.version}</span>
                : <Skel w="40px" h="11px" />
              }
              <span>© 2026</span>
            </div>
          </div>
        </footer>
      </div>
    </>
  );
}

/* ── Download card ─────────────────────────────────────────────────────── */

function DownloadCard({
  title,
  sub,
  glyph,
  ext,
  installer,
  recommended,
}: {
  title: string;
  sub: string;
  glyph?: ReactNode;
  ext?: string;
  installer?: Installer;
  recommended?: boolean;
}) {
  return (
    <div className={`dl-card${recommended ? " recommended" : ""}`}>
      {recommended && <span className="dl-badge">Recommended for you</span>}
      <div className="arch">
        {glyph ?? <Chip size={18} />}
        {title}
      </div>
      <div className="sub">{sub}</div>
      <div className="meta">
        <span className="fn">{installer?.filename ?? "—"}</span>
        <span>{installer ? `${fmtSize(installer.size)} · ${ext ?? ".dmg"}` : "—"}</span>
      </div>
      <a className="dl-btn" href={installer?.url} aria-disabled={!installer}>
        <Download size={16} />
        Download
      </a>
    </div>
  );
}

/* ── Popover Mockup (Mockup A — hero) ──────────────────────────────────── */

function PopoverMockup() {
  return (
    <div className="mockup popover-mockup" role="img" aria-label="TokenWatch menu-bar popover">
      {/* Top strip */}
      <div className="pop-topbar">
        <div className="pop-brand">
          <Eye size={18} />
          <span className="pop-wordmark">TokenWatch</span>
        </div>
        <span className="pop-updated">UPDATED 14:32</span>
      </div>

      {/* Gauges */}
      <div className="pop-gauges">
        {/* Session 5h gauge */}
        <div className="pop-gauge">
          <div className="pop-gauge-header">
            <span className="pop-gauge-label">SESSION 5H</span>
            <span className="pop-gauge-pct watch">62%</span>
          </div>
          <div className="pop-rail">
            <div className="pop-fill watch" style={{ width: "62%" }} />
            <div className="pop-tick" style={{ left: "70%" }} />
            <div className="pop-tick" style={{ left: "85%" }} />
            <div className="pop-tick end" style={{ left: "100%" }} />
            <div className="pop-pace" style={{ left: "58%" }} />
          </div>
          <div className="pop-gauge-meta">Resets in 2h 47m</div>
        </div>

        {/* Weekly gauge */}
        <div className="pop-gauge">
          <div className="pop-gauge-header">
            <span className="pop-gauge-label">WEEK</span>
            <span className="pop-gauge-pct safe">38%</span>
          </div>
          <div className="pop-rail">
            <div className="pop-fill safe" style={{ width: "38%" }} />
            <div className="pop-tick" style={{ left: "70%" }} />
            <div className="pop-tick" style={{ left: "85%" }} />
            <div className="pop-tick end" style={{ left: "100%" }} />
            <div className="pop-pace" style={{ left: "44%" }} />
          </div>
          <div className="pop-gauge-meta">Resets in 4d 6h</div>
        </div>
      </div>

      {/* Budgets section */}
      <div className="pop-section">
        <div className="pop-section-label">BUDGETS</div>
        <div className="pop-budget-row">
          <span className="pop-budget-name">Agents</span>
          <div className="pop-budget-rail">
            <div className="pop-budget-fill safe" style={{ width: "42%" }} />
          </div>
          <span className="pop-budget-cost safe">$4.20 <span className="est">est.</span></span>
        </div>
        <div className="pop-budget-row">
          <span className="pop-budget-name">Landing</span>
          <div className="pop-budget-rail">
            <div className="pop-budget-fill watch" style={{ width: "71%" }} />
          </div>
          <span className="pop-budget-cost watch">$8.91 <span className="est">est.</span></span>
        </div>
      </div>

      {/* Today by project */}
      <div className="pop-section">
        <div className="pop-section-label">TODAY BY PROJECT</div>
        <div className="pop-proj-row">
          <span className="pop-proj-name">algiers-v1</span>
          <div className="pop-proj-bar-wrap">
            <div className="pop-proj-bar" style={{ width: "48%" }} />
          </div>
          <span className="pop-proj-tokens">1.2M</span>
          <span className="pop-proj-pct">48%</span>
        </div>
        <div className="pop-proj-row">
          <span className="pop-proj-name">infra</span>
          <div className="pop-proj-bar-wrap">
            <div className="pop-proj-bar" style={{ width: "33%" }} />
          </div>
          <span className="pop-proj-tokens">820K</span>
          <span className="pop-proj-pct">33%</span>
        </div>
        <div className="pop-proj-row">
          <span className="pop-proj-name">landing</span>
          <div className="pop-proj-bar-wrap">
            <div className="pop-proj-bar" style={{ width: "19%" }} />
          </div>
          <span className="pop-proj-tokens">470K</span>
          <span className="pop-proj-pct">19%</span>
        </div>
      </div>

      {/* Command row */}
      <div className="pop-commands">
        <div className="pop-mute-toggle">
          <div className="pop-toggle-pill">
            <div className="pop-toggle-dot" />
          </div>
          <span className="pop-mute-label">Mute alerts</span>
        </div>
        <a className="pop-dashboard-btn" href="#dashboard">Dashboard</a>
      </div>
    </div>
  );
}

/* ── Dashboard Mockup (Mockup B — #dashboard section) ──────────────────── */

function DashboardMockup() {
  return (
    <div className="mockup dashboard-mockup" role="img" aria-label="TokenWatch dashboard de costos">
      {/* Titlebar */}
      <div className="mk-titlebar">
        <div className="mk-traffic">
          <span /><span /><span />
        </div>
        <span className="mk-conn">TokenWatch · Cost dashboard</span>
      </div>

      {/* Instrument readouts (inline, not boxed) */}
      <div className="dash-readouts">
        <div className="dash-readout">
          <span className="dash-num">12.4M</span>
          <span className="dash-readout-label">TOKENS IN RANGE</span>
        </div>
        <div className="dash-sep" />
        <div className="dash-readout">
          <span className="dash-num">$182.40</span>
          <span className="dash-readout-label">COST</span>
        </div>
        <div className="dash-sep" />
        <div className="dash-readout">
          <span className="dash-num">5</span>
          <span className="dash-readout-label">SERIES</span>
        </div>
        <div className="dash-sep" />
        <div className="dash-readout">
          <span className="dash-num">7d</span>
          <span className="dash-readout-label">RANGE</span>
        </div>
      </div>

      {/* Chart panel */}
      <div className="dash-chart-panel">
        {/* Y-axis labels */}
        <div className="dash-yaxis">
          <span>4M</span>
          <span>3M</span>
          <span>2M</span>
          <span>1M</span>
          <span>0</span>
        </div>
        {/* Chart area with inline SVG */}
        <div className="dash-chart-area">
          <svg viewBox="0 0 620 140" preserveAspectRatio="none" className="dash-chart-svg">
            {/* Grid lines */}
            <line x1="0" y1="35" x2="620" y2="35" stroke="rgba(255,255,255,0.05)" strokeWidth="1" />
            <line x1="0" y1="70" x2="620" y2="70" stroke="rgba(255,255,255,0.05)" strokeWidth="1" />
            <line x1="0" y1="105" x2="620" y2="105" stroke="rgba(255,255,255,0.05)" strokeWidth="1" />
            {/* Series 1 — accent (Opus) */}
            <path
              d="M0,120 C40,110 80,95 120,85 C160,75 200,60 240,55 C280,50 320,45 360,40 C400,35 440,38 480,32 C520,28 580,22 620,18 L620,140 L0,140 Z"
              fill="rgba(170,65,246,0.25)"
              stroke="#AA41F6"
              strokeWidth="1.5"
            />
            {/* Series 2 — info (Sonnet) */}
            <path
              d="M0,128 C40,122 80,118 120,112 C160,106 200,100 240,96 C280,92 320,88 360,85 C400,82 440,80 480,78 C520,76 580,72 620,70 L620,140 L0,140 Z"
              fill="rgba(96,165,250,0.18)"
              stroke="#60A5FA"
              strokeWidth="1.5"
            />
            {/* Series 3 — muted (Haiku) */}
            <path
              d="M0,135 C80,133 160,130 240,128 C320,126 400,124 480,122 C540,121 580,120 620,119 L620,140 L0,140 Z"
              fill="rgba(154,150,168,0.10)"
              stroke="rgba(154,150,168,0.4)"
              strokeWidth="1"
            />
            {/* Cursor probe line */}
            <line x1="400" y1="0" x2="400" y2="140" stroke="rgba(255,255,255,0.12)" strokeWidth="1" strokeDasharray="3,3" />
            <circle cx="400" cy="35" r="3" fill="#AA41F6" />
            <circle cx="400" cy="82" r="3" fill="#60A5FA" />
            <circle cx="400" cy="123" r="3" fill="rgba(154,150,168,0.7)" />
          </svg>
          {/* X-axis ticks */}
          <div className="dash-xaxis">
            <span>Jun 30</span>
            <span>Jul 1</span>
            <span>Jul 2</span>
            <span>Jul 3</span>
            <span>Jul 4</span>
            <span>Jul 5</span>
            <span>Jul 6</span>
          </div>
        </div>
      </div>

      {/* Ledger table */}
      <div className="dash-table">
        <div className="dash-table-head">
          <span>SERIES</span>
          <span className="num">JUN 30</span>
          <span className="num">JUL 1</span>
          <span className="num">JUL 2</span>
          <span className="num">JUL 3</span>
          <span className="num dash-hide-sm">JUL 4</span>
          <span className="num dash-hide-sm">TOTAL</span>
        </div>
        <div className="dash-table-row">
          <span className="dash-series-dot accent">Opus</span>
          <span className="num">1.2M</span>
          <span className="num">0.9M</span>
          <span className="num">1.4M</span>
          <span className="num">1.1M</span>
          <span className="num dash-hide-sm">1.3M</span>
          <span className="num dash-hide-sm">5.9M</span>
        </div>
        <div className="dash-table-row">
          <span className="dash-series-dot info">Sonnet</span>
          <span className="num">0.8M</span>
          <span className="num">0.6M</span>
          <span className="num">0.9M</span>
          <span className="num">0.7M</span>
          <span className="num dash-hide-sm">0.8M</span>
          <span className="num dash-hide-sm">3.8M</span>
        </div>
        <div className="dash-table-row">
          <span className="dash-series-dot muted">Haiku</span>
          <span className="num">0.4M</span>
          <span className="num">0.3M</span>
          <span className="num">0.5M</span>
          <span className="num">0.4M</span>
          <span className="num dash-hide-sm">0.5M</span>
          <span className="num dash-hide-sm">2.1M</span>
        </div>
        <div className="dash-table-row totals">
          <span>Total</span>
          <span className="num">2.4M</span>
          <span className="num">1.8M</span>
          <span className="num">2.8M</span>
          <span className="num">2.2M</span>
          <span className="num dash-hide-sm">2.6M</span>
          <span className="num dash-hide-sm">11.8M</span>
        </div>
      </div>
    </div>
  );
}
