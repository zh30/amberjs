import { useState } from "react";
import { useLang } from "../lib/i18n";
import {
  INSTALL_COMMANDS,
  useInstall,
  type InstallKey,
} from "../lib/install-choice";
import { landingCopy } from "../lib/landing-copy";

const KEYS: readonly InstallKey[] = ["unix", "windows", "source"];

function Lines({ lines }: { lines: readonly string[] }) {
  return (
    <>
      {lines.map((line, index) => (
        <span key={line}>
          {index > 0 ? <br /> : null}
          {line}
        </span>
      ))}
    </>
  );
}

export function InstallTabs() {
  const { lang } = useLang();
  const { key, setKey } = useInstall();
  const tabs = landingCopy[lang].tabs;
  return (
    <div className="tabs" role="tablist" aria-label={landingCopy[lang].installLabel}>
      {KEYS.map((item) => (
        <button
          key={item}
          type="button"
          role="tab"
          aria-selected={key === item}
          onClick={() => setKey(item)}
        >
          {tabs[item]}
        </button>
      ))}
    </div>
  );
}

export function CommandLine() {
  const { key } = useInstall();
  const command = INSTALL_COMMANDS[key];
  return (
    <p className="cmdline">
      <span className="wide">
        <Lines lines={command.wide} />
      </span>
      <span className="narrow">
        <Lines lines={command.narrow} />
      </span>
    </p>
  );
}

export function CopyInstall({ wide = false }: { wide?: boolean }) {
  const { lang } = useLang();
  const { key } = useInstall();
  const text = landingCopy[lang];
  const [label, setLabel] = useState<string | null>(null);
  const [bad, setBad] = useState(false);

  async function onCopy() {
    try {
      await navigator.clipboard.writeText(INSTALL_COMMANDS[key].copy);
      setBad(false);
      setLabel(text.copied);
    } catch {
      setBad(true);
      setLabel(text.copyFailed);
    }
    window.setTimeout(() => setLabel(null), 1600);
  }

  return (
    <button
      type="button"
      className={`solid honey${wide ? " copy-hero" : ""}${bad ? " bad" : ""}`}
      onClick={() => void onCopy()}
    >
      {label ?? text.copy}
    </button>
  );
}
