import { invoke as rawInvoke } from "@tauri-apps/api/core";
import { pushToast } from "./toast.svelte";

export interface PageNode {
  path: string;
  title: string;
  hasChildren: boolean;
  hasInk: boolean;
}

export interface PageContent {
  markdown: string;
  hasInk: boolean;
}

/** `[x, y, pressure, tiltX, tiltY, timestamp]` */
export type InkPoint = [number, number, number, number, number, number];

export interface InkStroke {
  points: InkPoint[];
  color: string;
  width: number;
}

export interface InkDocument {
  version: number;
  strokes: InkStroke[];
}

export interface SearchHit {
  path: string;
  title: string;
  snippet: string;
  score: number;
}

/**
 * Thin wrapper around `invoke` that surfaces any backend errors as toast
 * notifications. Returns `undefined` on error so callers can branch cleanly.
 */
export async function call<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | undefined> {
  try {
    return (await rawInvoke<T>(cmd, args)) as T;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(`[tauri] ${cmd} failed:`, msg);
    pushToast(msg, "error");
    return undefined;
  }
}

/**
 * Variant of `call` that throws on error. Use for hot paths like markdown
 * rendering where we don't want the toast spam.
 */
export async function callSilent<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return (await rawInvoke<T>(cmd, args)) as T;
}

export const cmd = {
  setVault: (path: string) => call<void>("set_vault", { path }),
  currentVault: () => call<string | null>("current_vault"),
  readPageTree: (virtualPath?: string) =>
    call<PageNode[]>("read_page_tree", { virtualPath }),
  readPage: (virtualPath: string) =>
    call<PageContent>("read_page", { virtualPath }),
  writePage: (virtualPath: string, markdown: string) =>
    call<void>("write_page", { virtualPath, markdown }),
  createPage: (parentPath: string, title: string) =>
    call<string>("create_page", { parentPath, title }),
  deletePage: (virtualPath: string) =>
    call<void>("delete_page", { virtualPath }),
  renamePage: (virtualPath: string, newTitle: string) =>
    call<string>("rename_page", { virtualPath, newTitle }),
  movePage: (virtualPath: string, newParent: string) =>
    call<string>("move_page", { virtualPath, newParent }),
  reorderSiblings: (parentPath: string, order: string[]) =>
    call<void>("reorder_siblings", { parentPath, order }),
  renderMarkdown: (markdown: string) =>
    callSilent<string>("render_markdown", { markdown }),
  readInk: (virtualPath: string) =>
    call<InkDocument | null>("read_ink", { virtualPath }),
  writeInk: (virtualPath: string, ink: InkDocument) =>
    call<void>("write_ink", { virtualPath, ink }),
  search: (query: string, limit?: number) =>
    call<SearchHit[]>("search", { query, limit }),
  rebuildIndex: () => call<void>("rebuild_index"),
  importAsset: (notePath: string, sourcePath: string) =>
    call<string>("import_asset", { notePath, sourcePath }),
};
