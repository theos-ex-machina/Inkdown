<script lang="ts">
  import { cmd, type PageNode, type SearchHit } from "../lib/tauri";
  import { debounce } from "../lib/debounce";

  type Props = {
    activePath: string | null;
    onSelect: (path: string) => void;
    refreshKey?: number;
  };

  let { activePath, onSelect, refreshKey = 0 }: Props = $props();

  type TreeEntry = {
    node: PageNode;
    expanded: boolean;
    children: TreeEntry[] | null;
  };

  let roots = $state<TreeEntry[]>([]);
  let editingPath = $state<string | null>(null);
  let editingValue = $state("");
  let creatingUnder = $state<string | null | undefined>(undefined); // path or null (=root) when active
  let creatingValue = $state("");
  let menu = $state<{ x: number; y: number; path: string } | null>(null);
  let dragPath = $state<string | null>(null);
  let dropTarget = $state<string | null | "root">(null);
  let searchQuery = $state("");
  let searchHits = $state<SearchHit[] | null>(null);

  async function loadChildren(parentVirtual: string | null): Promise<TreeEntry[]> {
    const nodes =
      (await cmd.readPageTree(parentVirtual ?? undefined)) ?? [];
    return nodes.map((n) => ({ node: n, expanded: false, children: null }));
  }

  async function refresh() {
    roots = await loadChildren(null);
  }

  $effect(() => {
    void refreshKey;
    refresh();
  });

  async function toggle(entry: TreeEntry) {
    if (!entry.node.hasChildren) return;
    entry.expanded = !entry.expanded;
    if (entry.expanded && entry.children === null) {
      entry.children = await loadChildren(entry.node.path);
    }
  }

  function findEntry(list: TreeEntry[], path: string): TreeEntry | null {
    for (const e of list) {
      if (e.node.path === path) return e;
      if (e.children) {
        const f = findEntry(e.children, path);
        if (f) return f;
      }
    }
    return null;
  }

  async function reloadParentOf(virtualPath: string) {
    const slash = virtualPath.lastIndexOf("/");
    if (slash === -1) {
      await refresh();
      return;
    }
    const parent = virtualPath.slice(0, slash);
    const parentEntry = findEntry(roots, parent);
    if (parentEntry) {
      parentEntry.children = await loadChildren(parent);
      parentEntry.expanded = true;
      // Backend's hasChildren is computed from the grandparent listing;
      // reflect the new child locally so the disclosure caret appears.
      if ((parentEntry.children?.length ?? 0) > 0) {
        parentEntry.node.hasChildren = true;
      }
    } else {
      await refresh();
    }
  }

  async function newAt(parent: string | null) {
    // The create-input only renders inside an expanded parent's children
    // block. So if the user invoked "New subpage" on a collapsed or
    // leaf-only page, expand it (loading children lazily) before showing
    // the input.
    if (parent !== null) {
      const entry = findEntry(roots, parent);
      if (entry) {
        if (entry.children === null) {
          entry.children = await loadChildren(parent);
        }
        entry.expanded = true;
      }
    }
    creatingUnder = parent;
    creatingValue = "Untitled";
    await Promise.resolve();
    const el = document.querySelector<HTMLInputElement>(".tree-create input");
    el?.focus();
    el?.select();
  }

  async function commitCreate(_e: Event) {
    if (creatingUnder === undefined) return;
    const title = creatingValue.trim();
    const parent = creatingUnder;
    creatingUnder = undefined;
    if (!title) return;
    const newPath = await cmd.createPage(parent ?? "", title);
    if (!newPath) return;
    if (parent) {
      await reloadParentOf(newPath);
    } else {
      await refresh();
    }
    onSelect(newPath);
  }

  function cancelCreate() {
    creatingUnder = undefined;
  }

  function beginRename(path: string, title: string) {
    editingPath = path;
    editingValue = title;
    queueMicrotask(() => {
      const el = document.querySelector<HTMLInputElement>(".tree-rename input");
      el?.focus();
      el?.select();
    });
  }

  async function commitRename() {
    if (!editingPath) return;
    const v = editingValue.trim();
    const oldPath = editingPath;
    editingPath = null;
    if (!v) return;
    const newPath = await cmd.renamePage(oldPath, v);
    if (!newPath) return;
    await reloadParentOf(newPath);
    if (activePath === oldPath) onSelect(newPath);
  }

  function cancelRename() {
    editingPath = null;
  }

  async function deleteAt(path: string) {
    const title = path.split("/").pop() ?? path;
    if (!confirm(`Delete "${title}"? Subpages and ink will be lost.`)) return;
    await cmd.deletePage(path);
    await reloadParentOf(path);
    if (activePath === path) onSelect("");
  }

  function openMenu(e: MouseEvent, path: string) {
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY, path };
  }
  function closeMenu() {
    menu = null;
  }

  async function performMenu(action: "new" | "rename" | "delete") {
    if (!menu) return;
    const path = menu.path;
    const entry = findEntry(roots, path);
    closeMenu();
    if (action === "new") {
      await newAt(path);
    } else if (action === "rename" && entry) {
      beginRename(path, entry.node.title);
    } else if (action === "delete") {
      await deleteAt(path);
    }
  }

  function onTreeKeyDown(e: KeyboardEvent) {
    if (!activePath) return;
    const entry = findEntry(roots, activePath);
    if (!entry) return;
    if (e.key === "F2") {
      e.preventDefault();
      beginRename(activePath, entry.node.title);
    } else if (e.key === "Delete") {
      e.preventDefault();
      void deleteAt(activePath);
    }
  }

  function onDragStart(e: DragEvent, path: string) {
    dragPath = path;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", path);
    }
  }
  function onDragOver(e: DragEvent, path: string | "root") {
    if (!dragPath) return;
    e.preventDefault();
    dropTarget = path;
  }
  function onDragLeave() {
    dropTarget = null;
  }
  async function onDrop(e: DragEvent, target: string | "root") {
    e.preventDefault();
    const src = dragPath;
    dragPath = null;
    dropTarget = null;
    if (!src) return;
    const newParent = target === "root" ? "" : target;
    if (src === newParent || newParent.startsWith(src + "/")) return;
    const moved = await cmd.movePage(src, newParent);
    if (!moved) return;
    await refresh();
    if (activePath === src) onSelect(moved);
  }

  const runSearch = debounce(async (q: string) => {
    if (!q.trim()) {
      searchHits = null;
      return;
    }
    searchHits = (await cmd.search(q.trim(), 50)) ?? [];
  }, 200);

  function onSearchInput(e: Event) {
    const v = (e.target as HTMLInputElement).value;
    searchQuery = v;
    runSearch(v);
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      searchQuery = "";
      searchHits = null;
    }
  }
</script>

<svelte:window onclick={closeMenu} />

<nav class="page-tree" onkeydown={onTreeKeyDown} aria-label="Page tree">
  <div
    class="tree-scroll"
    role="tree"
    tabindex="0"
    ondragover={(e) => onDragOver(e, "root")}
    ondrop={(e) => onDrop(e, "root")}
  >
    {#if creatingUnder === null}
      <div class="tree-create" style="padding-left: 12px">
        <input
          type="text"
          bind:value={creatingValue}
          onblur={commitCreate}
          onkeydown={(e) => {
            if (e.key === "Enter") commitCreate(e);
            else if (e.key === "Escape") cancelCreate();
          }}
        />
      </div>
    {/if}
    {#each roots as entry (entry.node.path)}
      {@render row(entry, 0)}
    {/each}
    {#if searchHits}
      <div class="search-results">
        <div class="search-header">Search results</div>
        {#each searchHits as hit (hit.path)}
          <button
            class="search-hit"
            class:active={hit.path === activePath}
            onclick={() => onSelect(hit.path)}
          >
            <div class="hit-title">{hit.title}</div>
            <div class="hit-path">{hit.path}</div>
            {#if hit.snippet}
              <div class="hit-snippet">{@html hit.snippet}</div>
            {/if}
          </button>
        {/each}
        {#if searchHits.length === 0}
          <div class="search-empty">No matches.</div>
        {/if}
      </div>
    {/if}
  </div>
  <div class="tree-footer">
    <input
      class="tree-search"
      type="search"
      placeholder="Search"
      value={searchQuery}
      oninput={onSearchInput}
      onkeydown={onSearchKey}
    />
    <button class="new-page" onclick={() => newAt(null)} title="New page (Ctrl+N)">+</button>
  </div>
</nav>

{#snippet row(entry: TreeEntry, depth: number)}
  <div
    class="tree-row"
    role="treeitem"
    tabindex="-1"
    aria-selected={entry.node.path === activePath}
    aria-expanded={entry.node.hasChildren ? entry.expanded : undefined}
    class:active={entry.node.path === activePath}
    class:dragging={dragPath === entry.node.path}
    class:drop={dropTarget === entry.node.path}
    style="padding-left: {12 + depth * 14}px"
    draggable="true"
    ondragstart={(e) => onDragStart(e, entry.node.path)}
    ondragover={(e) => onDragOver(e, entry.node.path)}
    ondragleave={onDragLeave}
    ondrop={(e) => onDrop(e, entry.node.path)}
    oncontextmenu={(e) => openMenu(e, entry.node.path)}
  >
    {#if entry.node.hasChildren}
      <button
        class="caret"
        class:open={entry.expanded}
        onclick={() => toggle(entry)}
        aria-label="Toggle"
      >
        ▸
      </button>
    {:else}
      <span class="caret placeholder"></span>
    {/if}
    {#if editingPath === entry.node.path}
      <span class="tree-rename">
        <input
          type="text"
          bind:value={editingValue}
          onblur={commitRename}
          onkeydown={(e) => {
            if (e.key === "Enter") commitRename();
            else if (e.key === "Escape") cancelRename();
          }}
        />
      </span>
    {:else}
      <button
        class="title"
        onclick={() => onSelect(entry.node.path)}
        ondblclick={() => beginRename(entry.node.path, entry.node.title)}
      >
        <span class="title-text">{entry.node.title}</span>
        {#if entry.node.hasInk}
          <span class="ink-dot" title="Has ink strokes"></span>
        {/if}
      </button>
    {/if}
  </div>
  {#if entry.expanded && entry.children}
    {#each entry.children as child (child.node.path)}
      {@render row(child, depth + 1)}
    {/each}
    {#if creatingUnder === entry.node.path}
      <div class="tree-create" style="padding-left: {12 + (depth + 1) * 14}px">
        <input
          type="text"
          bind:value={creatingValue}
          onblur={commitCreate}
          onkeydown={(e) => {
            if (e.key === "Enter") commitCreate(e);
            else if (e.key === "Escape") cancelCreate();
          }}
        />
      </div>
    {/if}
  {/if}
{/snippet}

{#if menu}
  <div class="context-menu" style="left: {menu.x}px; top: {menu.y}px" onclick={(e) => e.stopPropagation()} role="menu" tabindex="-1" onkeydown={() => {}}>
    <button onclick={() => performMenu("new")}>New subpage</button>
    <button onclick={() => performMenu("rename")}>Rename</button>
    <button class="danger" onclick={() => performMenu("delete")}>Delete</button>
  </div>
{/if}

<style>
  .page-tree {
    background: var(--mantle);
    border-right: 1px solid var(--surface0);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
  }
  .tree-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .tree-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 6px;
    color: var(--subtext1);
    font-size: 13px;
    line-height: 1.4;
    user-select: none;
    border-radius: 0;
  }
  .tree-row:hover {
    background: rgba(255, 255, 255, 0.025);
    color: var(--text);
  }
  .tree-row.active {
    background: var(--surface0);
    color: var(--text);
  }
  .tree-row.dragging {
    opacity: 0.5;
  }
  .tree-row.drop {
    background: var(--surface1);
  }
  .caret {
    width: 14px;
    text-align: center;
    color: var(--overlay0);
    font-size: 10px;
    transition: transform 80ms;
    flex-shrink: 0;
  }
  .caret.open {
    transform: rotate(90deg);
  }
  .caret.placeholder {
    display: inline-block;
  }
  .title {
    flex: 1;
    text-align: left;
    padding: 4px 4px 4px 2px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: inherit;
    min-width: 0;
  }
  .title-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ink-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mauve);
    flex-shrink: 0;
  }
  .tree-rename input,
  .tree-create input {
    flex: 1;
    background: var(--base);
    border: 1px solid var(--lavender);
    color: var(--text);
    padding: 2px 6px;
    border-radius: 4px;
    font: inherit;
    outline: none;
    min-width: 0;
  }
  .tree-footer {
    border-top: 1px solid var(--surface0);
    padding: 8px;
    display: flex;
    gap: 6px;
    background: var(--mantle);
  }
  .tree-search {
    flex: 1;
    background: var(--surface0);
    color: var(--text);
    border: 1px solid var(--surface1);
    border-radius: 4px;
    padding: 5px 8px;
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }
  .tree-search:focus {
    border-color: var(--lavender);
  }
  .new-page {
    background: var(--surface0);
    color: var(--text);
    width: 30px;
    border-radius: 4px;
    font-size: 16px;
    font-weight: 600;
    line-height: 1;
  }
  .new-page:hover {
    background: var(--surface1);
  }
  .search-results {
    margin-top: 8px;
    border-top: 1px solid var(--surface0);
    padding-top: 8px;
  }
  .search-header {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--overlay1);
    padding: 0 12px 4px;
  }
  .search-empty {
    padding: 6px 12px;
    color: var(--overlay1);
    font-size: 12px;
  }
  .search-hit {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    color: var(--subtext1);
    border-radius: 0;
  }
  .search-hit:hover {
    background: var(--surface0);
    color: var(--text);
  }
  .search-hit.active {
    background: var(--surface0);
    color: var(--text);
  }
  .hit-title {
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }
  .hit-path {
    color: var(--overlay1);
    font-size: 11px;
    font-family: var(--mono);
  }
  .hit-snippet {
    color: var(--subtext0);
    font-size: 12px;
    margin-top: 2px;
  }
  .hit-snippet :global(b) {
    color: var(--peach);
    font-weight: 600;
  }
  .context-menu {
    position: fixed;
    background: var(--surface0);
    border: 1px solid var(--surface1);
    border-radius: 6px;
    padding: 4px;
    z-index: 999;
    min-width: 160px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
  }
  .context-menu button {
    text-align: left;
    padding: 6px 10px;
    border-radius: 4px;
    color: var(--text);
    font-size: 12.5px;
  }
  .context-menu button:hover {
    background: var(--surface1);
  }
  .context-menu button.danger:hover {
    background: var(--red);
    color: var(--crust);
  }
</style>
