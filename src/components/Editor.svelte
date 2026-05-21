<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import { EditorView, keymap, drawSelection, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
  import { bracketMatching, indentOnInput } from "@codemirror/language";
  import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
  import { catppuccinExtension } from "../lib/cm-theme";
  import { livePreview, livePreviewBaseTheme } from "../lib/cm-live-preview";
  import { cmd } from "../lib/tauri";
  import { debounce } from "../lib/debounce";
  import { pushToast } from "../lib/toast.svelte";

  type Props = {
    virtualPath: string;
    onChange?: (markdownSrc: string) => void;
    onScroll?: (topLine: number) => void;
    onDirty?: (dirty: boolean) => void;
  };

  let { virtualPath, onChange, onScroll, onDirty }: Props = $props();

  let host: HTMLDivElement | null = $state(null);
  let view: EditorView | null = null;
  let currentPath = "";
  let loading = $state(true);
  const editableComp = new Compartment();

  const saveNow = debounce(async (path: string, src: string) => {
    await cmd.writePage(path, src);
    onDirty?.(false);
  }, 500);

  function emitScroll() {
    if (!view || !onScroll) return;
    const top = view.scrollDOM.scrollTop;
    const pos = view.lineBlockAtHeight(top);
    const line = view.state.doc.lineAt(pos.from).number - 1;
    onScroll(line);
  }

  async function loadPath(path: string) {
    if (!view || !path) return;
    currentPath = path;
    loading = true;
    const content = await cmd.readPage(path);
    if (!content) {
      loading = false;
      return;
    }
    if (currentPath !== path) return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: content.markdown },
    });
    view.scrollDOM.scrollTop = 0;
    onChange?.(content.markdown);
    loading = false;
  }

  onMount(() => {
    if (!host) return;

    const updateListener = EditorView.updateListener.of((u) => {
      if (u.docChanged) {
        const src = u.state.doc.toString();
        onChange?.(src);
        onDirty?.(true);
        if (currentPath) saveNow(currentPath, src);
      }
    });

    const scrollListener = EditorView.domEventHandlers({
      scroll() {
        emitScroll();
      },
    });

    const state = EditorState.create({
      doc: "",
      extensions: [
        history(),
        drawSelection(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        bracketMatching(),
        indentOnInput(),
        EditorView.lineWrapping,
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
        markdown({ base: markdownLanguage }),
        catppuccinExtension,
        livePreviewBaseTheme,
        livePreview,
        editableComp.of(EditorView.editable.of(true)),
        updateListener,
        scrollListener,
      ],
    });

    view = new EditorView({
      state,
      parent: host,
    });

    if (virtualPath) {
      void loadPath(virtualPath);
    }
  });

  let lastLoadedPath = "";
  $effect(() => {
    if (virtualPath && virtualPath !== lastLoadedPath && view) {
      saveNow.flush();
      lastLoadedPath = virtualPath;
      void loadPath(virtualPath);
    }
  });

  export function setEditable(editable: boolean) {
    if (!view) return;
    view.dispatch({
      effects: editableComp.reconfigure(EditorView.editable.of(editable)),
    });
  }

  export function scrollToLine(line: number) {
    if (!view) return;
    const safeLine = Math.max(1, Math.min(view.state.doc.lines, line + 1));
    const pos = view.state.doc.line(safeLine).from;
    view.dispatch({ effects: EditorView.scrollIntoView(pos, { y: "start" }) });
  }

  export function insertAtCursor(text: string) {
    if (!view) return;
    const sel = view.state.selection.main;
    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert: text },
      selection: { anchor: sel.from + text.length },
    });
  }

  export function focus() {
    view?.focus();
  }

  /**
   * Move the editor's text cursor to whatever document position lines up
   * with the supplied viewport coordinates. Used when the user clicks
   * through the ink overlay to get back into typing.
   */
  export function positionAtCoords(clientX: number, clientY: number) {
    if (!view) return;
    const pos = view.posAtCoords({ x: clientX, y: clientY });
    if (pos == null) return;
    view.focus();
    view.dispatch({ selection: { anchor: pos } });
  }

  async function onFileDrop(e: DragEvent) {
    if (!e.dataTransfer) return;
    const files = Array.from(e.dataTransfer.files ?? []);
    if (files.length === 0) return;
    e.preventDefault();
    for (const f of files) {
      const src = (f as unknown as { path?: string }).path;
      if (!src) {
        pushToast(
          "Drag from a file explorer (tauri exposes the OS path).",
          "info",
        );
        continue;
      }
      const ref = await cmd.importAsset(virtualPath, src);
      if (ref) insertAtCursor(ref + "\n");
    }
  }

  onDestroy(() => {
    saveNow.flush();
    view?.destroy();
    view = null;
  });
</script>

<div
  class="cm-host"
  role="textbox"
  tabindex="0"
  aria-label="Markdown editor"
  bind:this={host}
  ondragover={(e) => e.preventDefault()}
  ondrop={onFileDrop}
  aria-busy={loading}
></div>
