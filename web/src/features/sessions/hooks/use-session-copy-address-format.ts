import { useEffect, useState } from "react";

export type SessionCopyAddressFormat = "socks_url" | "http_url" | "host_port";

export const defaultSessionCopyAddressFormat: SessionCopyAddressFormat = "socks_url";
export const sessionCopyAddressFormatStorageKey = "proxy-broker.session-copy-address-format";

function isSessionCopyAddressFormat(value: string | null): value is SessionCopyAddressFormat {
  return value === "socks_url" || value === "http_url" || value === "host_port";
}

export function useSessionCopyAddressFormat() {
  const [copyAddressFormat, setCopyAddressFormat] = useState<SessionCopyAddressFormat>(() => {
    if (typeof window === "undefined") {
      return defaultSessionCopyAddressFormat;
    }
    const stored = window.localStorage.getItem(sessionCopyAddressFormatStorageKey);
    return isSessionCopyAddressFormat(stored) ? stored : defaultSessionCopyAddressFormat;
  });

  useEffect(() => {
    window.localStorage.setItem(sessionCopyAddressFormatStorageKey, copyAddressFormat);
  }, [copyAddressFormat]);

  return [copyAddressFormat, setCopyAddressFormat] as const;
}
