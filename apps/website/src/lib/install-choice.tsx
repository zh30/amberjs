import { createContext, useContext, useState, type ReactNode } from "react";

export const INSTALL_COMMANDS = {
  unix: {
    copy: "curl -fsSL https://get.amberjs.com/install.sh | sh",
    wide: ["curl -fsSL", "https://get.amberjs.com/install.sh | sh"],
    narrow: ["curl -fsSL", "https://get.amberjs.com/", "install.sh | sh"],
  },
  windows: {
    copy: "irm https://get.amberjs.com/install.ps1 | iex",
    wide: ["irm", "https://get.amberjs.com/install.ps1 | iex"],
    narrow: ["irm https://get.amberjs.com/", "install.ps1 | iex"],
  },
  source: {
    copy: "git clone https://github.com/zh30/amberjs && cd amberjs && cargo build --release",
    wide: [
      "git clone",
      "https://github.com/zh30/amberjs",
      "&& cd amberjs && cargo build --release",
    ],
    narrow: [
      "git clone",
      "https://github.com/zh30/amberjs",
      "&& cd amberjs",
      "&& cargo build --release",
    ],
  },
} as const;

export type InstallKey = keyof typeof INSTALL_COMMANDS;

interface InstallChoiceValue {
  key: InstallKey;
  setKey: (key: InstallKey) => void;
}

const InstallChoiceContext = createContext<InstallChoiceValue | null>(null);

export function InstallProvider({ children }: { children: ReactNode }) {
  const [key, setKey] = useState<InstallKey>("unix");
  return (
    <InstallChoiceContext.Provider value={{ key, setKey }}>
      {children}
    </InstallChoiceContext.Provider>
  );
}

export function useInstall(): InstallChoiceValue {
  const value = useContext(InstallChoiceContext);
  if (!value) {
    throw new Error("useInstall must be used within InstallProvider");
  }
  return value;
}
