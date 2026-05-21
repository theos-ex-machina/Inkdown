import { LazyStore } from "@tauri-apps/plugin-store";
import { cmd } from "./tauri";

const STORE_FILE = "config.json";
const VAULT_KEY = "vaultPath";

const store = new LazyStore(STORE_FILE);

class VaultState {
  /** `null` until we've consulted the store on startup. */
  path = $state<string | null | undefined>(undefined);
  loading = $state(true);

  async init() {
    this.loading = true;
    try {
      const saved = (await store.get<string>(VAULT_KEY)) ?? null;
      if (saved) {
        await cmd.setVault(saved);
        this.path = saved;
      } else {
        this.path = null;
      }
    } catch (e) {
      console.warn("[config] load failed:", e);
      this.path = null;
    } finally {
      this.loading = false;
    }
  }

  async set(newPath: string) {
    await cmd.setVault(newPath);
    await store.set(VAULT_KEY, newPath);
    await store.save();
    this.path = newPath;
  }

  async clear() {
    await store.delete(VAULT_KEY);
    await store.save();
    this.path = null;
  }
}

export const vault = new VaultState();
