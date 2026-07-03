import { APP_DISPLAY_NAME } from "@/platform/app-identity";

export function App() {
  return (
    <main style={{ padding: 24, fontFamily: "var(--font-stack)" }}>
      <h1>{APP_DISPLAY_NAME}</h1>
      <p>Menu-bar token monitor — scaffold. Features coming soon.</p>
    </main>
  );
}
