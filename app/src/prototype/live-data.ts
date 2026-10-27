import { useEffect, useRef, useState } from "react";
import { operate, type Agent } from "../api";

// Only safe operation results use this hook. TOTP uses authorization.ts directly.
export function useLiveData<T>(
  token: string,
  owner: Agent,
  operation: string,
  fields: object = {},
) {
  const key = JSON.stringify(fields);
  const [version, setVersion] = useState(0);
  const [data, setData] = useState<T>();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    const controller = new AbortController();
    setData(undefined);
    setLoading(true);
    setError("");
    operate<T>(token, owner, operation, JSON.parse(key), controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setData(value);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setError("Could not load this information. Refresh to try again.");
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [token, owner, operation, key, version]);
  return { data, error, loading, refresh: () => setVersion((n) => n + 1) };
}
export function useLivePages<T>(
  token: string,
  owner: Agent,
  operation: string,
  fields: object = {},
) {
  const key = JSON.stringify(fields);
  const [version, setVersion] = useState(0);
  const [items, setItems] = useState<T[]>([]);
  const [cursor, setCursor] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const active = useRef<AbortController | null>(null);
  const generation = useRef(0);
  useEffect(() => {
    const controller = new AbortController();
    active.current?.abort();
    active.current = controller;
    const current = ++generation.current;
    setItems([]);
    setCursor(null);
    setLoading(true);
    setError("");
    operate<{ items: T[]; next_cursor: number | null }>(
      token,
      owner,
      operation,
      { ...JSON.parse(key), page: { before: null, limit: 50 } },
      controller.signal,
    )
      .then((value) => {
        if (!Array.isArray(value.items)) throw new Error("Invalid page");
        if (!controller.signal.aborted && current === generation.current) {
          setItems(value.items);
          setCursor(value.next_cursor);
        }
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setError("Could not load these records. Refresh to try again.");
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => {
      controller.abort();
      active.current?.abort();
    };
  }, [token, owner, operation, key, version]);
  async function more() {
    if (loading || cursor === null) return;
    const current = generation.current,
      controller = new AbortController();
    active.current = controller;
    setLoading(true);
    setError("");
    try {
      const page = await operate<{ items: T[]; next_cursor: number | null }>(
        token,
        owner,
        operation,
        { ...JSON.parse(key), page: { before: cursor, limit: 50 } },
        controller.signal,
      );
      if (!Array.isArray(page.items)) throw new Error("Invalid page");
      if (!controller.signal.aborted && current === generation.current) {
        setItems((old) => [...old, ...page.items]);
        setCursor(page.next_cursor);
      }
    } catch {
      if (!controller.signal.aborted && current === generation.current)
        setError("Could not load more records. Try again.");
    } finally {
      if (!controller.signal.aborted && current === generation.current)
        setLoading(false);
    }
  }
  return {
    items,
    loading,
    error,
    more,
    hasMore: cursor !== null,
    refresh: () => setVersion((n) => n + 1),
  };
}
